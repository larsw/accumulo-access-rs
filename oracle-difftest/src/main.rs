// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

//! Differential test of the `accumulo-access` crate against the Lean oracle in `lean-oracle/`.
//!
//! The oracle's parser is proven to accept exactly the strings that the specification's grammar
//! derives, and to return the expression the grammar assigns. This harness streams inputs to the
//! oracle and checks, for every input, that the crate:
//! * accepts it exactly when the oracle does, apart from the crate's nesting limit;
//! * parses it to the same expression, up to the order and repetition of operands and the
//!   grouping of repeated `&` or `|`;
//! * evaluates it like the oracle for every subset of its tokens, through `check_authorization`,
//!   `check_authorization_csv` and `caching::check_authorization_csv`;
//! * displays it as a string that the oracle accepts and reads as the same expression.
//!
//! The protocol is described in `lean-oracle/Main.lean`.

use accumulo_access::{
    caching, check_authorization, check_authorization_csv, AuthorizationExpression, Lexer, Parser,
    ParserError,
};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;

const USAGE: &str = "\
usage: oracle-difftest [--oracle PATH] [--examples N] MODE

modes:
  exhaustive MAX_LEN [ALPHABET]  every string over ALPHABET of at most MAX_LEN characters
                                 (default alphabet: A B & | ( ) \" \\ space é)
  random COUNT [SEED]            random expressions from the grammar, half of them mutated
  upstream TESTDATA_JSON         Apache Accumulo Access's test vectors, also checked against
                                 their expected results

The oracle defaults to lean-oracle/.lake/build/bin/oracle (build it with `lake build` there).
Exits with 1 when the crate and the oracle disagree.";

/// The crate rejects parentheses nested deeper than this (`MAX_NESTING_DEPTH` in parser.rs). The
/// specification has no limit.
const CRATE_MAX_NESTING_DEPTH: usize = 256;

/// Truth tables cover expressions with at most this many distinct tokens.
const TABLE_CAP: usize = 6;

/// Apache's test vectors have expressions with up to 11 distinct tokens.
const UPSTREAM_TABLE_CAP: usize = 12;

fn main() -> ExitCode {
    match run_cli(std::env::args().skip(1).collect()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run_cli(mut args: Vec<String>) -> Result<bool, String> {
    let mut oracle = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../lean-oracle/.lake/build/bin/oracle"
    ));
    let mut examples = 5;
    while args.first().is_some_and(|arg| arg.starts_with("--")) {
        let option = args.remove(0);
        if args.is_empty() {
            return Err(format!("{option} needs a value"));
        }
        let value = args.remove(0);
        match option.as_str() {
            "--oracle" => oracle = PathBuf::from(value),
            "--examples" => examples = number(&value)?,
            _ => return Err(format!("unknown option {option}")),
        }
    }
    let mut report = Report::new(examples);
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["exhaustive", max_len, rest @ ..] if rest.len() <= 1 => {
            let max_len = number(max_len)?;
            let alphabet: Vec<char> = rest
                .first()
                .map_or("AB&|()\"\\ é".chars().collect(), |a| a.chars().collect());
            if alphabet.is_empty() {
                return Err("the alphabet is empty".into());
            }
            let inputs: Inputs = Arc::new(move || {
                Box::new(AllStrings::new(alphabet.clone(), max_len)) as Box<dyn Iterator<Item = _>>
            });
            run(&oracle, TABLE_CAP, inputs, |s, spec, shown| {
                compare(&mut report, s, &spec, &shown)
            })?;
        }
        ["random", count, rest @ ..] if rest.len() <= 1 => {
            let count = number(count)?;
            let seed = rest.first().map_or(Ok(1), |s| number(s))? as u64;
            let inputs: Inputs = Arc::new(move || {
                let mut rng = Rng(seed);
                Box::new((0..count).map(move |_| random_case(&mut rng)))
                    as Box<dyn Iterator<Item = _>>
            });
            run(&oracle, TABLE_CAP, inputs, |s, spec, shown| {
                compare(&mut report, s, &spec, &shown)
            })?;
        }
        ["upstream", path] => {
            let cases = Arc::new(upstream_cases(Path::new(path))?);
            let expressions = cases.clone();
            let inputs: Inputs = Arc::new(move || {
                let expressions = expressions.clone();
                Box::new((0..expressions.len()).map(move |i| expressions[i].expression.clone()))
                    as Box<dyn Iterator<Item = _>>
            });
            let mut next = cases.iter();
            run(&oracle, UPSTREAM_TABLE_CAP, inputs, |s, spec, shown| {
                let case = next.next().expect("one case per input");
                compare(&mut report, s, &spec, &shown);
                check_upstream(&mut report, case, &spec);
            })?;
        }
        _ => return Err("missing or unknown mode".into()),
    }
    Ok(report.print())
}

fn number(s: &str) -> Result<usize, String> {
    s.parse().map_err(|_| format!("not a number: {s}"))
}

// ----------------------------------------------------------------------------------------------
// Talking to the oracle

type Inputs = Arc<dyn Fn() -> Box<dyn Iterator<Item = String>> + Send + Sync>;

#[derive(Debug)]
enum Answer {
    Reject,
    Accept(Accepted),
}

#[derive(Debug)]
struct Accepted {
    depth: usize,
    tokens: Vec<String>,
    table: Option<Vec<bool>>,
    canon: String,
}

fn encode(s: &str) -> String {
    s.chars()
        .map(|c| u32::from(c).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_token(field: &str) -> Option<String> {
    field
        .split(',')
        .map(|n| n.parse().ok().and_then(char::from_u32))
        .collect()
}

fn parse_answer(line: &str) -> Result<Answer, String> {
    if line == "ERR" {
        return Ok(Answer::Reject);
    }
    let malformed = || format!("malformed answer from the oracle: {line:?}");
    let fields: Vec<&str> = line.split(' ').collect();
    let ["OK", depth, tokens, table, canon] = fields.as_slice() else {
        return Err(malformed());
    };
    let tokens = match *tokens {
        "-" => Vec::new(),
        tokens => tokens
            .split(';')
            .map(decode_token)
            .collect::<Option<_>>()
            .ok_or_else(malformed)?,
    };
    let table = match *table {
        "-" => None,
        table => Some(table.chars().map(|c| c == '1').collect()),
    };
    Ok(Answer::Accept(Accepted {
        depth: depth.parse().map_err(|_| malformed())?,
        tokens,
        table,
        canon: canon.to_string(),
    }))
}

fn parse_with_crate(s: &str) -> Result<AuthorizationExpression, ParserError> {
    Parser::new(Lexer::new(s)).parse()
}

/// Sends every input to the oracle, followed by the crate's `Display` output for it (or the
/// input again when the crate rejects it), and calls `each` with the oracle's two answers.
///
/// A writer thread feeds the oracle while this thread reads its answers, so neither side can
/// block the other on a full pipe. Both threads produce the inputs from `inputs`, which must
/// yield the same sequence every time.
fn run(
    oracle: &Path,
    table_cap: usize,
    inputs: Inputs,
    mut each: impl FnMut(&str, Answer, Answer),
) -> Result<(), String> {
    let mut child = Command::new(oracle)
        .arg(table_cap.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start the oracle {}: {e}", oracle.display()))?;
    let stdin = child.stdin.take().expect("piped stdin");
    let writer_inputs = inputs.clone();
    let writer = std::thread::spawn(move || -> std::io::Result<()> {
        let mut out = BufWriter::new(stdin);
        for s in writer_inputs() {
            let shown = parse_with_crate(&s).map_or_else(|_| s.clone(), |e| e.to_string());
            writeln!(out, "{}", encode(&s))?;
            writeln!(out, "{}", encode(&shown))?;
        }
        out.flush()
    });
    let mut lines = BufReader::new(child.stdout.take().expect("piped stdout")).lines();
    let mut next_answer = || -> Result<Answer, String> {
        let line = lines
            .next()
            .ok_or("the oracle stopped answering")?
            .map_err(|e| format!("cannot read from the oracle: {e}"))?;
        parse_answer(&line)
    };
    for (i, s) in inputs().enumerate() {
        let spec = next_answer()?;
        let shown = next_answer()?;
        each(&s, spec, shown);
        if (i + 1) % 1_000_000 == 0 {
            eprintln!("{} inputs", i + 1);
        }
    }
    writer
        .join()
        .map_err(|_| "the writer thread panicked".to_string())?
        .map_err(|e| format!("cannot write to the oracle: {e}"))?;
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("the oracle exited with {status}"));
    }
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// Comparing

struct Report {
    examples: usize,
    inputs: u64,
    accepted: u64,
    over_limit: u64,
    failures: BTreeMap<&'static str, (u64, Vec<String>)>,
}

impl Report {
    fn new(examples: usize) -> Self {
        Report {
            examples,
            inputs: 0,
            accepted: 0,
            over_limit: 0,
            failures: BTreeMap::new(),
        }
    }

    fn fail(&mut self, kind: &'static str, input: &str, detail: impl FnOnce() -> String) {
        let (count, examples) = self.failures.entry(kind).or_default();
        *count += 1;
        if examples.len() < self.examples {
            examples.push(format!("{input:?}: {}", detail()));
        }
    }

    /// Prints the report and returns whether everything agreed.
    fn print(&self) -> bool {
        println!("inputs:                         {}", self.inputs);
        println!("valid per the spec:             {}", self.accepted);
        println!(
            "valid but nested beyond {CRATE_MAX_NESTING_DEPTH}:   {}",
            self.over_limit
        );
        if self.failures.is_empty() {
            println!("\nno disagreements");
            return true;
        }
        println!("\ndisagreements:");
        for (kind, (count, examples)) in &self.failures {
            println!("\n{count} × {kind}");
            for example in examples {
                println!("    {example}");
            }
        }
        false
    }
}

fn compare(report: &mut Report, s: &str, spec: &Answer, shown: &Answer) {
    report.inputs += 1;
    let rust = parse_with_crate(s);
    let accepted = match spec {
        Answer::Reject => {
            if let Ok(e) = &rust {
                report.fail("crate accepts an input the spec rejects", s, || {
                    format!("parsed as {e:?}")
                });
            }
            return;
        }
        Answer::Accept(accepted) => accepted,
    };
    report.accepted += 1;
    if accepted.depth > CRATE_MAX_NESTING_DEPTH {
        report.over_limit += 1;
        if rust.is_ok() {
            report.fail("crate accepts nesting deeper than its limit", s, || {
                format!("depth {}", accepted.depth)
            });
        }
        return;
    }
    let expr = match rust {
        Ok(expr) => expr,
        Err(err) => {
            report.fail("crate rejects an input the spec accepts", s, || {
                format!("{err:?}")
            });
            return;
        }
    };
    let got = canon(&expr);
    if got != accepted.canon {
        report.fail("crate parses an input differently from the spec", s, || {
            format!("crate {got}, spec {}", accepted.canon)
        });
        return;
    }
    if let Some(table) = &accepted.table {
        check_evaluation(report, s, &accepted.tokens, table);
    }
    match shown {
        Answer::Accept(d) if d.canon == accepted.canon => {}
        Answer::Accept(d) => report.fail(
            "crate displays an expression as one that means something else",
            s,
            || format!("shown as {expr}, which the spec reads as {}", d.canon),
        ),
        Answer::Reject => report.fail(
            "crate displays an expression as text the spec rejects",
            s,
            || format!("shown as {expr}"),
        ),
    }
}

fn check_evaluation(report: &mut Report, s: &str, tokens: &[String], table: &[bool]) {
    let csv_safe = tokens.iter().all(|t| !t.contains(','));
    for (mask, &expected) in table.iter().enumerate() {
        let auths: Vec<String> = tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| mask >> i & 1 == 1)
            .map(|(_, t)| t.clone())
            .collect();
        let got = check_authorization(s, &auths);
        if got != Ok(expected) {
            report.fail(
                "crate evaluates an expression differently (check_authorization)",
                s,
                || format!("holding {auths:?}: crate {got:?}, spec {expected}"),
            );
            return;
        }
        if !csv_safe {
            continue;
        }
        let csv = auths.join(",");
        let got = check_authorization_csv(s.to_string(), csv.clone());
        if got != Ok(expected) {
            report.fail(
                "crate evaluates an expression differently (check_authorization_csv)",
                s,
                || format!("holding {csv:?}: crate {got:?}, spec {expected}"),
            );
            return;
        }
        if mask == 0 || mask == table.len() - 1 {
            let got = caching::check_authorization_csv(s.to_string(), csv.clone());
            if got != Ok(expected) {
                report.fail(
                    "crate evaluates an expression differently (caching::check_authorization_csv)",
                    s,
                    || format!("holding {csv:?}: crate {got:?}, spec {expected}"),
                );
                return;
            }
        }
    }
}

/// The crate's expression in the oracle's canonical form (see `canon` in
/// `lean-oracle/AccessOracle/Canon.lean`).
fn canon(expr: &AuthorizationExpression) -> String {
    match expr {
        AuthorizationExpression::Nil => "EMPTY".to_string(),
        expr => canon_node(expr),
    }
}

fn canon_node(expr: &AuthorizationExpression) -> String {
    match expr {
        AuthorizationExpression::AccessToken(v) => {
            let code_points: Vec<String> = v.chars().map(|c| u32::from(c).to_string()).collect();
            format!("[{}]", code_points.join(","))
        }
        AuthorizationExpression::ConjunctionOf(_) => group('&', expr),
        AuthorizationExpression::DisjunctionOf(_) => group('|', expr),
        // Never valid inside an expression, so it can never match the oracle.
        AuthorizationExpression::Nil => "NIL".to_string(),
    }
}

fn group(op: char, expr: &AuthorizationExpression) -> String {
    fn collect(op: char, expr: &AuthorizationExpression, out: &mut Vec<String>) {
        match (op, expr) {
            ('&', AuthorizationExpression::ConjunctionOf(nodes))
            | ('|', AuthorizationExpression::DisjunctionOf(nodes)) => {
                nodes.iter().for_each(|node| collect(op, node, out))
            }
            _ => out.push(canon_node(expr)),
        }
    }
    let mut operands = Vec::new();
    collect(op, expr, &mut operands);
    operands.sort();
    operands.dedup();
    match operands.as_slice() {
        [single] => single.clone(),
        operands => format!("{op}({})", operands.join(",")),
    }
}

// ----------------------------------------------------------------------------------------------
// Apache's test vectors

struct UpstreamCase {
    expression: String,
    expected: String,
    auth_sets: Vec<Vec<String>>,
}

fn upstream_cases(path: &Path) -> Result<Vec<UpstreamCase>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let malformed = || format!("unexpected layout in {}", path.display());
    let strings = |v: &serde_json::Value| -> Option<Vec<String>> {
        v.as_array()?
            .iter()
            .map(|s| s.as_str().map(String::from))
            .collect()
    };
    let mut cases = Vec::new();
    for set in json.as_array().ok_or_else(malformed)? {
        let auth_sets: Vec<Vec<String>> = set["auths"]
            .as_array()
            .ok_or_else(malformed)?
            .iter()
            .map(strings)
            .collect::<Option<_>>()
            .ok_or_else(malformed)?;
        for test in set["tests"].as_array().ok_or_else(malformed)? {
            let expected = test["expectedResult"].as_str().ok_or_else(malformed)?;
            for expression in strings(&test["expressions"]).ok_or_else(malformed)? {
                cases.push(UpstreamCase {
                    expression,
                    expected: expected.to_string(),
                    auth_sets: auth_sets.clone(),
                });
            }
        }
    }
    Ok(cases)
}

/// Checks the oracle and the crate against the expected result. An entity with several
/// authorization sets has access when every set grants it, as in Apache's `AccessEvaluator`.
fn check_upstream(report: &mut Report, case: &UpstreamCase, spec: &Answer) {
    let s = case.expression.as_str();
    let spec_result = match spec {
        Answer::Reject => Some("ERROR"),
        Answer::Accept(accepted) => accepted.table.as_ref().map(|table| {
            let grants = |set: &Vec<String>| {
                let mask = accepted
                    .tokens
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| set.contains(t))
                    .fold(0, |mask, (i, _)| mask | 1 << i);
                table[mask]
            };
            if case.auth_sets.iter().all(grants) {
                "ACCESSIBLE"
            } else {
                "INACCESSIBLE"
            }
        }),
    };
    match spec_result {
        None => report.fail(
            "oracle has too many tokens for a truth table",
            s,
            String::new,
        ),
        Some(result) if result != case.expected => {
            report.fail("oracle disagrees with Apache's test vectors", s, || {
                format!("expected {}, oracle {result}", case.expected)
            })
        }
        Some(_) => {}
    }
    let crate_result = if parse_with_crate(s).is_err() {
        "ERROR"
    } else if case
        .auth_sets
        .iter()
        .all(|set| check_authorization(s, set) == Ok(true))
    {
        "ACCESSIBLE"
    } else {
        "INACCESSIBLE"
    };
    if crate_result != case.expected {
        report.fail("crate disagrees with Apache's test vectors", s, || {
            format!("expected {}, crate {crate_result}", case.expected)
        });
    }
}

// ----------------------------------------------------------------------------------------------
// Inputs

/// Every string over `alphabet` of at most `max_len` characters, shortest first.
struct AllStrings {
    alphabet: Vec<char>,
    max_len: usize,
    digits: Vec<usize>,
    done: bool,
}

impl AllStrings {
    fn new(alphabet: Vec<char>, max_len: usize) -> Self {
        AllStrings {
            alphabet,
            max_len,
            digits: Vec::new(),
            done: false,
        }
    }
}

impl Iterator for AllStrings {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.done {
            return None;
        }
        let s = self.digits.iter().map(|&d| self.alphabet[d]).collect();
        // Advance like an odometer; when every digit wraps, move on to the next length.
        let mut i = self.digits.len();
        loop {
            if i == 0 {
                if self.digits.len() == self.max_len {
                    self.done = true;
                } else {
                    self.digits = vec![0; self.digits.len() + 1];
                }
                break;
            }
            i -= 1;
            self.digits[i] += 1;
            if self.digits[i] < self.alphabet.len() {
                break;
            }
            self.digits[i] = 0;
        }
        Some(s)
    }
}

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

/// Few distinct names, so that expressions repeat tokens.
const NAMES: &[&str] = &["A", "B", "C", "a_b", "x-y.z", "1:2/3"];
const TOKEN_CHARS: &str = "ABCabcxyz019_-.:/";
const QUOTED_CHARS: &[char] = &[
    'a', 'Z', ' ', '&', '|', '(', ')', ',', '"', '\\', 'é', '五', '🕺', '\t', '\n', '\r', '\0',
    '\u{7f}', '\u{80}', '\u{fffd}',
];
const MUTATION_CHARS: &[char] = &[
    '&', '|', '(', ')', '"', '\\', ' ', 'A', 'é', '\n', ',', '\t', '*', '\0',
];

fn random_case(rng: &mut Rng) -> String {
    let mut s = match rng.below(100) {
        0 => String::new(),
        // Around the crate's nesting limit.
        1 | 2 => {
            let depth = CRATE_MAX_NESTING_DEPTH - 4 + rng.below(9);
            let inner = random_expression(rng, 5);
            format!("{}{inner}{}", "(".repeat(depth), ")".repeat(depth))
        }
        _ => random_expression(rng, 0),
    };
    if rng.chance(50) {
        mutate(rng, &mut s);
    }
    s
}

fn random_expression(rng: &mut Rng, depth: usize) -> String {
    let operands = 1 + rng.below(if depth < 3 { 4 } else { 2 });
    let op = if rng.chance(50) { "&" } else { "|" };
    let parts: Vec<String> = (0..operands)
        .map(|_| {
            if depth < 6 && rng.chance(30) {
                format!("({})", random_expression(rng, depth + 1))
            } else {
                random_token(rng)
            }
        })
        .collect();
    parts.join(op)
}

fn random_token(rng: &mut Rng) -> String {
    let value: String = match rng.below(10) {
        0..=5 => rng.pick(NAMES).to_string(),
        6 => {
            let chars: Vec<char> = TOKEN_CHARS.chars().collect();
            (0..1 + rng.below(6)).map(|_| rng.pick(&chars)).collect()
        }
        _ => (0..1 + rng.below(5))
            .map(|_| rng.pick(QUOTED_CHARS))
            .collect(),
    };
    let needs_quotes = !value
        .chars()
        .all(|c| TOKEN_CHARS.contains(c) || c.is_ascii_alphanumeric());
    if needs_quotes || rng.chance(20) {
        let escaped: String = value
            .chars()
            .flat_map(|c| match c {
                '"' | '\\' => vec!['\\', c],
                c => vec![c],
            })
            .collect();
        format!("\"{escaped}\"")
    } else {
        value
    }
}

fn mutate(rng: &mut Rng, s: &mut String) {
    let mut chars: Vec<char> = s.chars().collect();
    for _ in 0..1 + rng.below(3) {
        let at = rng.below(chars.len() + 1);
        match rng.below(4) {
            0 => chars.insert(at, rng.pick(MUTATION_CHARS)),
            1 if at < chars.len() => {
                chars.remove(at);
            }
            2 if at < chars.len() => chars[at] = rng.pick(MUTATION_CHARS),
            3 if at + 1 < chars.len() => chars.swap(at, at + 1),
            _ => {}
        }
    }
    *s = chars.into_iter().collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_strings_counts_every_length() {
        let all: Vec<String> = AllStrings::new(vec!['a', 'b'], 2).collect();
        assert_eq!(all, ["", "a", "b", "aa", "ab", "ba", "bb"]);
    }

    #[test]
    fn canon_ignores_order_repetition_and_grouping() {
        let a = canon(&parse_with_crate("B&(A&B)").unwrap());
        let b = canon(&parse_with_crate("A&B").unwrap());
        assert_eq!(a, b);
        assert_eq!(a, "&([65],[66])");
        assert_eq!(canon(&parse_with_crate("").unwrap()), "EMPTY");
        assert_eq!(
            canon(&parse_with_crate("(A|B)&C").unwrap()),
            "&([67],|([65],[66]))"
        );
    }

    #[test]
    fn answers_round_trip() {
        assert_eq!(encode("A\n"), "65 10");
        let Answer::Accept(a) = parse_answer("OK 1 65;66,67 0001 &([65],[66,67])").unwrap() else {
            panic!("expected an accepted answer");
        };
        assert_eq!(a.tokens, ["A", "BC"]);
        assert_eq!(a.table, Some(vec![false, false, false, true]));
        assert!(matches!(parse_answer("ERR"), Ok(Answer::Reject)));
        assert!(parse_answer("OK 1").is_err());
    }
}
