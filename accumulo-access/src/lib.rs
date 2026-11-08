// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

pub mod authorization_expression;
mod authorizations;
#[cfg(feature = "caching")]
pub mod caching;
mod lexer;
mod parser;

pub use crate::authorization_expression::AuthorizationExpression;
pub use crate::authorizations::Authorizations;
pub use crate::lexer::Lexer;
pub use crate::parser::Parser;
pub use crate::parser::ParserError;

use std::collections::HashSet;

// Runs the README's code blocks as doc-tests, so its examples keep compiling against the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[derive(Debug)]
pub enum JsonError {
    ParsingFailed(String),
}

// implement Display for JsonError
impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            JsonError::ParsingFailed(e) => write!(f, "{}", e),
        }
    }
}

pub struct AccessEvaluator {}

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
///    let expression = "label1|label5";
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
    let result = auth_expr.evaluate(&authorized_labels);
    Ok(result)
}

// Prepares a function that can be used to check if the given set of access tokens authorizes access to the resource which protection is described by the given expression.
pub fn prepare_authorization_csv(tokens: String) -> impl Fn(String) -> Result<bool, ParserError> {
    // An empty field (from "", "A," or "A,,B") is not an access token.
    let authorized_labels: HashSet<String> = tokens
        .split(',')
        .filter(|token| !token.is_empty())
        .map(String::from)
        .collect();
    move |expression| {
        let auth_expr = Parser::new(Lexer::new(&expression)).parse()?;
        Ok(auth_expr.evaluate(&authorized_labels))
    }
}

/// Checks if the given set of access tokens authorizes access to the resource which protection is described by the given expression.
pub fn check_authorization_csv(expression: String, tokens: String) -> Result<bool, ParserError> {
    prepare_authorization_csv(tokens)(expression)
}

pub fn expression_to_json_string(expression: &str) -> Result<String, ParserError> {
    let lexer: Lexer<'_> = Lexer::new(expression);
    let mut parser = Parser::new(lexer);

    let auth_expr = parser.parse();

    match auth_expr {
        Ok(auth_expr) => Ok(auth_expr.to_json_str()),
        Err(e) => Err(e),
    }
}

pub fn expression_to_json(expression: &str) -> Result<serde_json::Value, JsonError> {
    let lexer: Lexer<'_> = Lexer::new(expression);
    let mut parser = Parser::new(lexer);
    let expr = parser.parse();
    match expr {
        Ok(expr) => Ok(expr.to_json()),
        Err(e) => Err(JsonError::ParsingFailed(format!("{:?}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("", "", true)]
    #[case("label1", "label1", true)]
    #[case("label1|label2", "label1", true)]
    #[case("label1&label2", "label1", false)]
    #[case("label1&label2", "label1,label2", true)]
    #[case("label1&(label2|label3)", "label1", false)]
    #[case("label1&(label2|label3)", "label1,label3", true)]
    #[case("label1&(label2|label3)", "label1,label2", true)]
    #[case("(label2|label3)", "label1", false)]
    #[case("(label2|label3)", "label2", true)]
    #[case("(label2&label3)", "label2", false)]
    #[case("((label2|label3))", "label2", true)]
    #[case("((label2&label3))", "label2", false)]
    #[case("(((((label2&label3)))))", "label2", false)]
    #[case("\"a b c\"", "\"a b c\"", true)]
    #[case("\"abc!12\"&\"abc\\\\xyz\"&GHI", "abc\\xyz,abc!12", false)] // Taken from SPECIFICATION.md
    #[case("label1|label2", "", false)]
    #[case("", "anything", true)]
    #[case("((label1|label2)&(label3|label4))", "label1,label3", true)]
    #[case("\"标签A\"|label2", "标签A", true)]
    fn test_check_authorization(
        #[case] expr: impl AsRef<str>,
        #[case] authorized_tokens: impl AsRef<str>,
        #[case] expected: bool,
    ) {
        let authorized_tokens: Vec<String> = AsRef::as_ref(&authorized_tokens)
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string().replace(['"', '\''], ""))
            .collect();

        let result = check_authorization(expr.as_ref(), &authorized_tokens).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn normalization_test() {
        let expression = "A&B&A&(D|E)&(E|D)"; // -> A&B&(D|E)
        let lexer: Lexer<'_> = Lexer::new(expression);
        let mut parser = Parser::new(lexer);

        let mut auth_expr = parser.parse().unwrap();
        auth_expr.normalize();
        let expected = AuthorizationExpression::ConjunctionOf(vec![
            AuthorizationExpression::AccessToken("A".to_string()),
            AuthorizationExpression::AccessToken("B".to_string()),
            AuthorizationExpression::DisjunctionOf(vec![
                AuthorizationExpression::AccessToken("D".to_string()),
                AuthorizationExpression::AccessToken("E".to_string()),
            ]),
        ]);
        assert_eq!(expected, auth_expr);
        // `==` ignores order and duplicates, so also check the exact normalized form.
        assert_eq!(auth_expr.to_string(), "(D|E)&A&B");
    }

    #[test]
    fn test_check_authorization_invalid_expression() {
        let expr = "label1&&label2";
        let tokens: Vec<String> = vec!["label1".to_string(), "label2".to_string()];
        let result = check_authorization(expr, &tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_expression_to_json_string_and_json() {
        let expr = "A&(B|C)";
        let json_str = expression_to_json_string(expr).unwrap();
        let json_val = expression_to_json(expr).unwrap();
        assert!(json_str.contains("\"and\""));
        assert!(json_val["and"].is_array());
    }

    #[test]
    fn test_prepare_authorization_csv_and_check() {
        let tokens = "A,B,C".to_string();
        let checker = prepare_authorization_csv(tokens);
        assert_eq!(checker("A&B".to_string()), Ok(true));
        assert_eq!(checker("A&D".to_string()), Ok(false));
    }

    #[test]
    fn test_check_authorization_csv() {
        let expr = "A|B";
        let tokens = "B,C".to_string();
        assert_eq!(check_authorization_csv(expr.to_string(), tokens), Ok(true));
    }
}
