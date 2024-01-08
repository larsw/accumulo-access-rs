// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

mod lexer;
mod parser;
#[cfg(feature = "caching")]
pub mod caching;

pub use crate::lexer::Lexer;
pub use crate::parser::Parser;
pub use crate::parser::ParserError;

/// Checks if the given set of access tokens authorizes access to the resource which protection is described by the given expression.
///
/// Arguments:
/// * `expression` - The expression to parse and evaluate.
/// * `tokens` - The set of access tokens to check.
///
/// Returns:
/// * `Ok(true)` if the expression is valid and the tokens are authorized.
/// * `Ok(false)` if the expression is valid and the tokens are not authorized.
/// * `Err(ParserError)` if the expression is invalid.
///
/// # Examples
/// ```
/// use accumulo_access::check_authorization;
///
///    let expression = "label1 | label5";
///    let tokens = &Vec::from([
///      String::from("label1"),
///      String::from("label5"),
///      String::from("label 🕺"),
///    ]);
///
///    let result = match check_authorization(expression, tokens) {
///     Ok(result) => {
///         assert_eq!(result, true);
///     }
///     Err(_) => panic!("Unexpected error"),
///    };
/// ```
pub fn check_authorization(expression: &str, tokens: &[String]) -> Result<bool, ParserError> {
    let lexer: Lexer<'_> = Lexer::new(expression);
    let mut parser = Parser::new(lexer);

    let auth_expr = parser.parse()?;
    let authorized_labels = tokens.iter().cloned().collect();
    println!("{}, {:?}", auth_expr.to_json_str(), authorized_labels);
    let result = auth_expr.evaluate(&authorized_labels);
    Ok(result)
}

pub fn check_authorization_csv(
    expression: String,
    tokens: String,
) -> Result<bool, ParserError> {
    let tokens: Vec<String> = tokens.split(',').map(|s| s.to_string()).collect();
    check_authorization(expression.as_str(), &tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("label1", "label1", true)]
    #[case("label1|label2", "label1", true)]
    #[case("label1&label2", "label1", false)]
    #[case("label1&label2", "label1,label2", true)]
    #[case("label1&(label2 | label3)", "label1", false)]
    #[case("label1&(label2 | label3)", "label1,label3", true)]
    #[case("label1&(label2 | label3)", "label1,label2", true)]
    #[case("(label2 | label3)", "label1", false)]
    #[case("(label2 | label3)", "label2", true)]
    #[case("(label2 & label3)", "label2", false)]
    #[case("((label2 | label3))", "label2", true)]
    #[case("((label2 & label3))", "label2", false)]
    #[case("(((((label2 & label3)))))", "label2", false)]
    fn test_check_authorization(
        #[case] expr: impl AsRef<str>,
        #[case] authorized_tokens: impl AsRef<str>,
        #[case] expected: bool,
    ) {
        let authorized_tokens: Vec<String> = authorized_tokens
            .as_ref()
            .to_owned()
            .split(',')
            .map(|s| s.to_string())
            .collect();

        let result = check_authorization(expr.as_ref(), &authorized_tokens).unwrap();
        assert_eq!(result, expected);
    }
}
