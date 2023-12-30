// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

mod lexer;
mod parser;

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
/// fn main() {
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
/// }
/// ```
pub fn check_authorization(expression: &str, tokens: &Vec<String>) -> Result<bool, ParserError> {
    let lexer: Lexer<'_> = Lexer::new(&expression);
    let mut parser = Parser::new(lexer);

    let auth_expr = parser.parse().map_err(|e| e.into())?;
    let authorized_labels = tokens.iter().cloned().collect();
    println!("{}, {:?}", auth_expr.to_json_str(), authorized_labels);
    let result = auth_expr.evaluate(&authorized_labels);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_authorization() {
        let expression = "label1 | label5";
        let tokens = &Vec::from([
            String::from("label1"),
            String::from("label5"),
            String::from("label 🕺"),
        ]);
        let result = check_authorization(expression, tokens);
        match result {
            Ok(result) => assert_eq!(result, true),
            Err(_) => panic!("Unexpected error"),
        }
    }
}
