use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fmt::{Display, Formatter, Write};
use std::hash::{Hash, Hasher};

use crate::lexer::is_allowed_char_for_unquoted_access_token;

#[derive(Debug, Clone)]
pub enum AuthorizationExpression {
    /// A conjunction of multiple access tokens or scopes.
    ConjunctionOf(Vec<AuthorizationExpression>),
    /// A disjunction of multiple access tokens or scopes.
    DisjunctionOf(Vec<AuthorizationExpression>),
    /// An access token.
    AccessToken(String),
    /// A nil expression (empty string).
    Nil,
}

impl Hash for AuthorizationExpression {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.set_hash());
    }
}

/// Orders expressions structurally: `Nil` first, then `&` and `|` expressions (by their
/// operands, then `&` before `|`), then access tokens. This is a total order that agrees with
/// `==` on normalized expressions.
impl Ord for AuthorizationExpression {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (AuthorizationExpression::AccessToken(a), AuthorizationExpression::AccessToken(b)) => {
                a.cmp(b)
            }
            (
                AuthorizationExpression::ConjunctionOf(a)
                | AuthorizationExpression::DisjunctionOf(a),
                AuthorizationExpression::ConjunctionOf(b)
                | AuthorizationExpression::DisjunctionOf(b),
            ) => a.cmp(b).then_with(|| self.rank().cmp(&other.rank())),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

impl PartialOrd for AuthorizationExpression {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for AuthorizationExpression {}

impl PartialEq for AuthorizationExpression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                AuthorizationExpression::ConjunctionOf(a),
                AuthorizationExpression::ConjunctionOf(b),
            ) => {
                let self_set: HashSet<_> = a.iter().collect();
                let other_set: HashSet<_> = b.iter().collect();
                self_set == other_set
            }
            (
                AuthorizationExpression::DisjunctionOf(a),
                AuthorizationExpression::DisjunctionOf(b),
            ) => {
                let self_set: HashSet<_> = a.iter().collect();
                let other_set: HashSet<_> = b.iter().collect();
                self_set == other_set
            }
            (AuthorizationExpression::AccessToken(a), AuthorizationExpression::AccessToken(b)) => {
                a == b
            }
            (AuthorizationExpression::Nil, AuthorizationExpression::Nil) => true,
            _ => false,
        }
    }
}

/// Writes the expression in the access expression syntax, parenthesizing nested `&` and `|`
/// expressions and quoting access tokens that cannot appear unquoted.
impl Display for AuthorizationExpression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorizationExpression::Nil => Ok(()),
            AuthorizationExpression::ConjunctionOf(nodes) => Self::fmt_nodes(nodes, '&', f),
            AuthorizationExpression::DisjunctionOf(nodes) => Self::fmt_nodes(nodes, '|', f),
            AuthorizationExpression::AccessToken(token)
                if !token.is_empty()
                    && token.chars().all(is_allowed_char_for_unquoted_access_token) =>
            {
                f.write_str(token)
            }
            AuthorizationExpression::AccessToken(token) => {
                f.write_char('"')?;
                for c in token.chars() {
                    if c == '"' || c == '\\' {
                        f.write_char('\\')?;
                    }
                    f.write_char(c)?;
                }
                f.write_char('"')
            }
        }
    }
}

impl AuthorizationExpression {
    /// Create a new `AuthorizationExpression` from a JSON value.
    ///
    /// # Arguments
    /// json - The JSON value to parse.
    ///
    /// # Returns
    /// A new `AuthorizationExpression` instance.
    ///
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let json = serde_json::json!({
    ///   "and": [
    ///    "A",
    ///   {
    ///     "or": [
    ///       "B",
    ///      "C"
    ///    ]
    ///  }
    /// ]
    /// });
    /// let expr = AuthorizationExpression::from_json(&json).unwrap();
    /// ```
    pub fn from_json(json: &serde_json::Value) -> Result<Self, String> {
        match json {
            // `to_json` represents the empty expression as `null`.
            serde_json::Value::Null => Ok(AuthorizationExpression::Nil),
            _ => Self::node_from_json(json),
        }
    }

    /// Like `from_json`, but only accepts what can appear inside an expression: an object with
    /// a single "and" or "or" key, or a non-empty access token.
    fn node_from_json(json: &serde_json::Value) -> Result<Self, String> {
        match json {
            serde_json::Value::Object(obj) => match (obj.len(), obj.get("and"), obj.get("or")) {
                (1, Some(nodes), _) => Ok(AuthorizationExpression::ConjunctionOf(
                    Self::nodes_from_json(nodes)?,
                )),
                (1, _, Some(nodes)) => Ok(AuthorizationExpression::DisjunctionOf(
                    Self::nodes_from_json(nodes)?,
                )),
                _ => Err("Invalid JSON object".to_string()),
            },
            serde_json::Value::String(token) if !token.is_empty() => {
                Ok(AuthorizationExpression::AccessToken(token.to_string()))
            }
            _ => Err("Invalid JSON value".to_string()),
        }
    }

    fn nodes_from_json(json: &serde_json::Value) -> Result<Vec<Self>, String> {
        match json.as_array() {
            Some(nodes) if !nodes.is_empty() => nodes.iter().map(Self::node_from_json).collect(),
            _ => Err("Invalid JSON object: the operands must be a non-empty array".to_string()),
        }
    }

    /// Evaluate the expression with the given set of authorizations.
    /// Returns `true` if the authorizations are valid, `false` otherwise.
    ///
    /// # Arguments
    /// authorizations - The set of authorizations to check.
    ///
    /// # Example
    /// ```
    /// use std::collections::HashSet;
    /// use accumulo_access::AuthorizationExpression;
    /// let expr = AuthorizationExpression::ConjunctionOf(vec![
    ///    AuthorizationExpression::AccessToken("A".to_string()),
    ///   AuthorizationExpression::DisjunctionOf(vec![
    ///      AuthorizationExpression::AccessToken("B".to_string()),
    ///     AuthorizationExpression::AccessToken("C".to_string()),
    /// ]),
    /// ]);
    /// let authorizations = HashSet::from([
    ///    "A".to_string(),
    ///   "B".to_string(),
    /// ]);
    /// assert_eq!(expr.evaluate(&authorizations), true);
    /// ```
    pub fn evaluate(&self, authorizations: &HashSet<String>) -> bool {
        match self {
            AuthorizationExpression::Nil => true,

            AuthorizationExpression::ConjunctionOf(nodes) => {
                nodes.iter().all(|node| node.evaluate(authorizations))
            }

            AuthorizationExpression::DisjunctionOf(nodes) => {
                nodes.iter().any(|node| node.evaluate(authorizations))
            }

            AuthorizationExpression::AccessToken(token) => authorizations.contains(token),
        }
    }

    /// Create a JSON representation of the expression tree.
    ///
    /// # Returns
    /// A JSON value representing the expression tree.
    ///
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let expr = AuthorizationExpression::ConjunctionOf(vec![
    ///   AuthorizationExpression::AccessToken("A".to_string()),
    ///  AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    /// let json = expr.to_json();
    /// assert_eq!(json, serde_json::json!({"and": ["A", "B"]}));
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            AuthorizationExpression::Nil => serde_json::Value::Null,
            AuthorizationExpression::ConjunctionOf(nodes) => {
                let mut json = serde_json::json!({"and": []});
                let and = json.as_object_mut().unwrap().get_mut("and").unwrap();
                for node in nodes {
                    and.as_array_mut().unwrap().push(node.to_json());
                }
                json
            }
            AuthorizationExpression::DisjunctionOf(nodes) => {
                let mut json = serde_json::json!({"or": []});
                let or = json.as_object_mut().unwrap().get_mut("or").unwrap();
                for node in nodes {
                    or.as_array_mut().unwrap().push(node.to_json());
                }
                json
            }
            AuthorizationExpression::AccessToken(token) => serde_json::json!(token),
        }
    }

    /// Create a JSON string representation of the expression tree.
    ///
    /// # Returns
    /// A JSON string representing the expression tree.
    ///
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let expr = AuthorizationExpression::ConjunctionOf(vec![
    ///  AuthorizationExpression::AccessToken("A".to_string()),
    ///  AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    ///
    /// let json_str = expr.to_json_str();
    /// assert_eq!(json_str, "{\"and\":[\"A\",\"B\"]}");
    pub fn to_json_str(&self) -> String {
        self.to_json().to_string()
    }

    /// Create a string representation of the expression tree.
    ///
    /// # Returns
    /// A string representing the expression tree.
    ///
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let expr1 = AuthorizationExpression::ConjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    ///
    /// let expr_str = expr1.to_expression_str();
    /// assert_eq!(expr_str, "A&B");
    ///
    /// let expr2 = AuthorizationExpression::DisjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// ]);
    ///
    /// let expr_str = expr2.to_expression_str();
    /// assert_eq!(expr_str, "A|B");
    pub fn to_expression_str(&self) -> String {
        self.to_string()
    }

    /// Normalize the expression tree by sorting and deduplicating the nodes.
    ///
    /// # Example
    /// ```
    /// use accumulo_access::AuthorizationExpression;
    /// let mut expr = AuthorizationExpression::ConjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// AuthorizationExpression::DisjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("C".to_string()),
    /// AuthorizationExpression::AccessToken("D".to_string()),
    /// AuthorizationExpression::AccessToken("D".to_string())]
    /// )]);
    /// expr.normalize();
    /// let expected = AuthorizationExpression::ConjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("A".to_string()),
    /// AuthorizationExpression::AccessToken("B".to_string()),
    /// AuthorizationExpression::DisjunctionOf(vec![
    /// AuthorizationExpression::AccessToken("C".to_string()),
    /// AuthorizationExpression::AccessToken("D".to_string())]
    /// )]);
    ///
    /// assert_eq!(expr, expected);
    /// // `==` ignores order and duplicates; the normalized form is sorted and deduplicated.
    /// assert_eq!(expr.to_string(), "(C|D)&A&B");
    pub fn normalize(&mut self) {
        if let AuthorizationExpression::ConjunctionOf(nodes)
        | AuthorizationExpression::DisjunctionOf(nodes) = self
        {
            // Normalize the operands first, so that equal operands become identical and end up
            // next to each other when sorted.
            for node in nodes.iter_mut() {
                node.normalize();
            }
            nodes.sort();
            nodes.dedup();
        }
    }

    /// Hashes the expression the way `PartialEq` compares it: ignoring the order and duplicates
    /// of the operands of `&` and `|`.
    fn set_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hasher.write_u8(self.rank());
        match self {
            AuthorizationExpression::ConjunctionOf(nodes)
            | AuthorizationExpression::DisjunctionOf(nodes) => {
                let mut hashes: Vec<u64> = nodes.iter().map(Self::set_hash).collect();
                hashes.sort_unstable();
                hashes.dedup();
                hashes.hash(&mut hasher);
            }
            AuthorizationExpression::AccessToken(token) => token.hash(&mut hasher),
            AuthorizationExpression::Nil => {}
        }
        hasher.finish()
    }

    fn fmt_nodes(
        nodes: &[AuthorizationExpression],
        operator: char,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        for (i, node) in nodes.iter().enumerate() {
            if i > 0 {
                f.write_char(operator)?;
            }
            match node {
                AuthorizationExpression::ConjunctionOf(_)
                | AuthorizationExpression::DisjunctionOf(_) => write!(f, "({})", node)?,
                _ => write!(f, "{}", node)?,
            }
        }
        Ok(())
    }

    /// The position of the variant in the `Ord` order.
    fn rank(&self) -> u8 {
        match self {
            AuthorizationExpression::Nil => 0,
            AuthorizationExpression::ConjunctionOf(_) => 1,
            AuthorizationExpression::DisjunctionOf(_) => 2,
            AuthorizationExpression::AccessToken(_) => 3,
        }
    }
}

// test for normalize
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, Parser};

    fn parse(input: &str) -> AuthorizationExpression {
        Parser::new(Lexer::new(input)).parse().unwrap()
    }

    fn hash(expr: &AuthorizationExpression) -> u64 {
        let mut hasher = DefaultHasher::new();
        expr.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn some_basic_equality_and_ordering_tests() {
        assert_eq!(
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("A".to_string())
        );
        assert_ne!(
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string())
        );

        assert_eq!(
            AuthorizationExpression::ConjunctionOf(vec![
                AuthorizationExpression::AccessToken("A".to_string()),
                AuthorizationExpression::AccessToken("B".to_string()),
            ]),
            AuthorizationExpression::ConjunctionOf(vec![
                AuthorizationExpression::AccessToken("B".to_string()),
                AuthorizationExpression::AccessToken("A".to_string()),
            ])
        );

        assert_eq!(
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("A".to_string()),
                AuthorizationExpression::AccessToken("B".to_string()),
            ]),
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("B".to_string()),
                AuthorizationExpression::AccessToken("A".to_string()),
            ])
        );
    }

    #[test]
    fn new_expr_from_json() {
        let json = serde_json::json!({
            "and": [
                "A",
                {
                    "or": [
                        "B",
                        "C"
                    ]
                }
            ]
        });
        let expr = AuthorizationExpression::from_json(&json).unwrap();
        assert_eq!(
            expr,
            AuthorizationExpression::ConjunctionOf(vec![
                AuthorizationExpression::AccessToken("A".to_string()),
                AuthorizationExpression::DisjunctionOf(vec![
                    AuthorizationExpression::AccessToken("B".to_string()),
                    AuthorizationExpression::AccessToken("C".to_string()),
                ]),
            ])
        );
    }

    #[test]
    fn test_normalize1() {
        let mut expr = AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("C".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("D".to_string()),
            ]),
        ]);

        expr.normalize();

        assert_eq!(
            expr,
            AuthorizationExpression::ConjunctionOf(vec![
                AuthorizationExpression::AccessToken("B".to_string()),
                AuthorizationExpression::AccessToken("A".to_string()),
                AuthorizationExpression::DisjunctionOf(vec![
                    AuthorizationExpression::AccessToken("D".to_string()),
                    AuthorizationExpression::AccessToken("C".to_string()),
                ]),
            ])
        );
        // `==` ignores order and duplicates, so also check the exact normalized form.
        assert_eq!(expr.to_string(), "(C|D)&A&B");
    }

    #[test]
    fn test_nil_evaluates_true() {
        let expr = AuthorizationExpression::Nil;
        let set = std::collections::HashSet::new();
        assert!(expr.evaluate(&set));
    }

    #[test]
    fn test_access_token_evaluates_false() {
        let expr = AuthorizationExpression::AccessToken("X".to_string());
        let set = std::collections::HashSet::from(["Y".to_string()]);
        assert!(!expr.evaluate(&set));
    }

    #[test]
    fn test_disjunction_evaluates_false() {
        let expr = AuthorizationExpression::DisjunctionOf(vec![
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
        ]);
        let set = std::collections::HashSet::from(["C".to_string()]);
        assert!(!expr.evaluate(&set));
    }

    #[test]
    fn test_json_roundtrip() {
        let expr = AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("B".to_string()),
                AuthorizationExpression::AccessToken("C".to_string()),
            ]),
        ]);
        let json = expr.to_json();
        let expr2 = AuthorizationExpression::from_json(&json).unwrap();
        assert_eq!(expr, expr2);
    }

    #[test]
    fn test_to_expression_str_and_display() {
        let expr = AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
        ]);
        assert_eq!(expr.to_expression_str(), "A&B");
        assert_eq!(format!("{}", expr), "A&B");
    }

    #[test]
    fn test_to_json_str() {
        let expr = AuthorizationExpression::AccessToken("A".to_string());
        let json_str = expr.to_json_str();
        assert_eq!(json_str, "\"A\"");
    }

    #[test]
    fn test_to_expression_str_round_trips() {
        for input in [
            "A&(B|C)",
            "(A|B)&(C|D)",
            "\"a b\"&\"x\\\"y\\\\z\"",
            "\"a|b\"",
            "\"tab\there\"|\"nul\0\"",
        ] {
            let expr = parse(input);
            assert_eq!(parse(&expr.to_expression_str()), expr, "{}", input);
        }
        assert_eq!(parse("\"a|b\"").to_expression_str(), "\"a|b\"");
    }

    #[test]
    fn test_from_json_rejects_what_expressions_cannot_express() {
        for json in [
            serde_json::json!({"and": "A"}),
            serde_json::json!({"or": 5}),
            serde_json::json!({"and": []}),
            serde_json::json!({"and": ["A"], "or": ["B"]}),
            serde_json::json!({"or": ["A", null]}),
            serde_json::json!(""),
        ] {
            assert!(
                AuthorizationExpression::from_json(&json).is_err(),
                "{}",
                json
            );
        }
    }

    #[test]
    fn test_nil_json_roundtrip() {
        let json = AuthorizationExpression::Nil.to_json();
        assert_eq!(
            AuthorizationExpression::from_json(&json),
            Ok(AuthorizationExpression::Nil)
        );
    }

    #[test]
    fn test_normalize_dedups_equal_operands_in_any_order() {
        let mut expr = parse("(A|C)&(A|B)&(C|A)");
        expr.normalize();
        match &expr {
            AuthorizationExpression::ConjunctionOf(nodes) => assert_eq!(nodes.len(), 2),
            _ => panic!("unexpected {:?}", expr),
        }
        assert_eq!(expr, parse("(A|B)&(A|C)"));
    }

    #[test]
    fn test_hash_and_ord_are_consistent_with_eq() {
        let a = parse("(X|X|Y)&Z");
        let b = parse("(Y|X)&Z");
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));

        let and = parse("A&B");
        let or = parse("A|B");
        assert_ne!(and, or);
        assert_ne!(and.cmp(&or), Ordering::Equal);
        assert_eq!(AuthorizationExpression::Nil.cmp(&and), Ordering::Less);
    }
}
