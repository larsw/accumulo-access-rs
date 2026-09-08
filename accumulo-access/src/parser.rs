// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

use crate::authorization_expression::AuthorizationExpression;
use crate::lexer::{Lexer, Operator, Token};
use std::iter::Peekable;
use thiserror::Error;

/// `ParserError` is returned when the parser encounters an error.
#[derive(Error, Debug, PartialEq, Clone)]
pub enum ParserError {
    /// A parenthesised scope is empty, e.g. `()`.
    EmptyScope,
    /// Two operands appear next to each other with no operator between them.
    MissingOperator,
    /// The parser encountered an unexpected token.
    UnexpectedToken(Token),
    /// The expression ended while an access token, a scope or a `)` was still expected.
    UnexpectedEndOfExpression,
    /// The parser encountered a mix of operators ('&' and '|') in one scope.
    MixingOperators,
    /// The parser encountered a lexer error.
    LexerError(crate::lexer::LexerError),
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::EmptyScope => write!(f, "Empty scope"),
            ParserError::MissingOperator => write!(f, "Missing operator"),
            ParserError::UnexpectedToken(token) => write!(f, "Unexpected token: {}", token),
            ParserError::UnexpectedEndOfExpression => write!(f, "Unexpected end of expression"),
            ParserError::MixingOperators => write!(f, "Mixing operators"),
            ParserError::LexerError(e) => write!(f, "{}", e),
        }
    }
}

/// `Parser` is used to parse an expression and return an `AuthorizationExpression`-based tree.
///
/// It is a recursive-descent parser for the grammar in the
/// [AccessExpression specification](https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md):
///
/// ```abnf
/// access-expression = [expression]  ; only the expression as a whole may be empty
/// expression        = (access-token / paren-expression) [and-expression / or-expression]
/// paren-expression  = "(" expression ")"
/// and-expression    = "&" (access-token / paren-expression) [and-expression]
/// or-expression     = "|" (access-token / paren-expression) [or-expression]
/// ```
pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
}

impl<'a> Parser<'a> {
    /// Creates a new `Parser` instance.
    ///
    /// # Arguments
    ///
    /// * `lexer` - The `Lexer` instance to use for tokenization.
    pub fn new(lexer: Lexer<'a>) -> Self {
        Parser {
            lexer: lexer.peekable(),
        }
    }

    /// Parse the input string and return an AuthorizationExpression.
    /// If the input string is invalid, a ParserError is returned.
    ///
    /// # Example
    /// ```
    ///  use std::collections::HashSet;
    ///  use accumulo_access::{Lexer, Parser};
    ///  let input = "label1&label5&(label3|label8|\"label 🕺\")";
    ///  let lexer: Lexer<'_> = Lexer::new(input);
    ///  let mut parser = Parser::new(lexer);
    ///  let ast = parser.parse().unwrap();
    ///  let authorized_tokens : &HashSet<String> = &[
    ///    String::from("label1"),
    ///    String::from("label5"),
    ///    String::from("label 🕺"),
    ///  ].iter().cloned().collect();
    ///  assert_eq!(ast.evaluate(&authorized_tokens), true);
    /// ```
    pub fn parse(&mut self) -> Result<AuthorizationExpression, ParserError> {
        // access-expression = [expression]. The empty string is a valid access
        // expression, and it authorizes everything.
        if self.peek()?.is_none() {
            return Ok(AuthorizationExpression::Nil);
        }

        let expression = self.parse_expression()?;

        // A trailing `)` has no scope to close, e.g. `A)`.
        match self.next_token()? {
            None => Ok(expression),
            Some(token) => Err(ParserError::UnexpectedToken(token)),
        }
    }

    /// `expression = (access-token / paren-expression) [and-expression / or-expression]`
    fn parse_expression(&mut self) -> Result<AuthorizationExpression, ParserError> {
        let first = self.parse_operand()?;

        let operator = match self.peek()? {
            Some(Token::And) => Operator::Conjunction,
            Some(Token::Or) => Operator::Disjunction,
            Some(Token::AccessToken(_) | Token::OpenParen) => {
                return Err(ParserError::MissingOperator)
            }
            // `None` ends the expression, `)` ends the enclosing scope; either
            // way this is a single operand and our caller deals with the rest.
            _ => return Ok(first),
        };

        let mut nodes = vec![first];
        loop {
            match self.peek()? {
                Some(Token::And) if operator == Operator::Conjunction => {}
                Some(Token::Or) if operator == Operator::Disjunction => {}
                // "Once a `&` is seen, then can only have `&` and not `|`,
                // unless using parenthesis" -- and the same the other way round.
                Some(Token::And | Token::Or) => return Err(ParserError::MixingOperators),
                Some(Token::AccessToken(_) | Token::OpenParen) => {
                    return Err(ParserError::MissingOperator)
                }
                _ => break,
            }
            self.next_token()?;
            nodes.push(self.parse_operand()?);
        }

        Ok(match operator {
            Operator::Conjunction => AuthorizationExpression::ConjunctionOf(nodes),
            Operator::Disjunction => AuthorizationExpression::DisjunctionOf(nodes),
        })
    }

    /// `access-token / paren-expression` -- the operand an expression, a `&` or
    /// a `|` must be followed by.
    fn parse_operand(&mut self) -> Result<AuthorizationExpression, ParserError> {
        match self.next_token()? {
            Some(Token::AccessToken(value)) => Ok(AuthorizationExpression::AccessToken(value)),
            Some(Token::OpenParen) => {
                // paren-expression = "(" expression ")". Unlike the expression
                // as a whole, a parenthesised one may not be empty.
                if let Some(Token::CloseParen) = self.peek()? {
                    return Err(ParserError::EmptyScope);
                }
                let inner = self.parse_expression()?;
                match self.next_token()? {
                    Some(Token::CloseParen) => Ok(inner),
                    Some(token) => Err(ParserError::UnexpectedToken(token)),
                    None => Err(ParserError::UnexpectedEndOfExpression),
                }
            }
            // An expression must start with an access token or a scope, so a
            // leading or doubled operator lands here (`&A`, `A&&B`).
            Some(token) => Err(ParserError::UnexpectedToken(token)),
            // An access token or a scope must follow an operator (`A&`).
            None => Err(ParserError::UnexpectedEndOfExpression),
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, ParserError> {
        match self.lexer.next() {
            Some(Ok(token)) => Ok(Some(token)),
            Some(Err(e)) => Err(ParserError::LexerError(e)),
            None => Ok(None),
        }
    }

    fn peek(&mut self) -> Result<Option<&Token>, ParserError> {
        match self.lexer.peek() {
            Some(Ok(token)) => Ok(Some(token)),
            Some(Err(e)) => Err(ParserError::LexerError(e.clone())),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse(expression: &str) -> Result<AuthorizationExpression, ParserError> {
        Parser::new(Lexer::new(expression)).parse()
    }

    fn token(value: &str) -> AuthorizationExpression {
        AuthorizationExpression::AccessToken(value.to_string())
    }

    #[rstest]
    // The four "Examples of Proper Expressions" from the specification.
    #[case("BLUE")]
    #[case("RED&BLUE")]
    #[case("RED&BLUE&GREEN")]
    #[case("(RED&BLUE)|(GREEN&(PINK|PURPLE))")]
    // Nesting and quoting.
    #[case("((label2|label3))")]
    #[case("(((((label2&label3)))))")]
    #[case("label1&label5&(label3|label8|\"label 🕺\")")]
    #[case("\"abc!12\"&\"abc\\\\xyz\"&GHI")]
    #[case("A&(B|C)&D")]
    #[case("(A|B)&(C|D)")]
    fn accepts_valid_expressions(#[case] expression: &str) {
        assert!(
            parse(expression).is_ok(),
            "expected {expression:?} to parse, got {:?}",
            parse(expression)
        );
    }

    #[test]
    fn accepts_the_empty_expression() {
        assert_eq!(Ok(AuthorizationExpression::Nil), parse(""));
    }

    #[rstest]
    // The four "Examples of Improper Expressions" from the specification.
    #[case("&BLUE", ParserError::UnexpectedToken(Token::And))]
    #[case("(RED&BLUE)|", ParserError::UnexpectedEndOfExpression)]
    #[case("RED&BLUE|GREEN", ParserError::MixingOperators)]
    #[case("RED|BLUE&GREEN", ParserError::MixingOperators)]
    // Trailing operator.
    #[case("label1&", ParserError::UnexpectedEndOfExpression)]
    #[case("label1|", ParserError::UnexpectedEndOfExpression)]
    #[case("label1&label2&", ParserError::UnexpectedEndOfExpression)]
    #[case("(A&B", ParserError::UnexpectedEndOfExpression)]
    // Leading operator.
    #[case("|label1", ParserError::UnexpectedToken(Token::Or))]
    #[case("(&label1)", ParserError::UnexpectedToken(Token::And))]
    // Repeated operator.
    #[case("label1&&label2", ParserError::UnexpectedToken(Token::And))]
    #[case("label1||label2", ParserError::UnexpectedToken(Token::Or))]
    #[case("label1&|label2", ParserError::UnexpectedToken(Token::Or))]
    // Unbalanced parentheses.
    #[case("(label1", ParserError::UnexpectedEndOfExpression)]
    #[case("(label1|label2", ParserError::UnexpectedEndOfExpression)]
    #[case("label1&(label2", ParserError::UnexpectedEndOfExpression)]
    #[case("label1)", ParserError::UnexpectedToken(Token::CloseParen))]
    #[case("(label1))", ParserError::UnexpectedToken(Token::CloseParen))]
    #[case("A&B)", ParserError::UnexpectedToken(Token::CloseParen))]
    // Empty scope: `paren-expression` requires an `expression` inside.
    #[case("()", ParserError::EmptyScope)]
    #[case("(())", ParserError::EmptyScope)]
    #[case("A&()", ParserError::EmptyScope)]
    // Two operands with no operator between them.
    #[case("A(B)", ParserError::MissingOperator)]
    #[case("(A)(B)", ParserError::MissingOperator)]
    #[case("A\"B\"", ParserError::MissingOperator)]
    #[case("A&B(C)", ParserError::MissingOperator)]
    fn rejects_invalid_expressions(#[case] expression: &str, #[case] expected: ParserError) {
        assert_eq!(Err(expected), parse(expression), "for {expression:?}");
    }

    #[test]
    fn mixing_operators_is_allowed_across_scopes() {
        assert!(parse("RED&(BLUE|GREEN)").is_ok());
        assert!(parse("RED|(BLUE&GREEN)").is_ok());
    }

    #[rstest]
    // Whitespace is not part of the access-token character set, so it fails in
    // the lexer rather than reading as a missing operator.
    #[case("[", crate::lexer::LexerError::UnexpectedCharacter('[', 1))]
    #[case("A&B C", crate::lexer::LexerError::UnexpectedCharacter(' ', 4))]
    #[case("A & B", crate::lexer::LexerError::UnexpectedCharacter(' ', 2))]
    fn a_lexer_error_is_reported_as_such(
        #[case] expression: &str,
        #[case] expected: crate::lexer::LexerError,
    ) {
        assert_eq!(
            Err(ParserError::LexerError(expected)),
            parse(expression),
            "for {expression:?}"
        );
    }

    #[test]
    fn operands_keep_their_source_order() {
        assert_eq!(
            Ok(AuthorizationExpression::ConjunctionOf(vec![
                token("A"),
                token("B"),
                token("C"),
            ])),
            parse("A&B&C")
        );
        // `PartialEq` on the tree is order-insensitive, so compare the
        // serialised form to actually pin the order down.
        assert_eq!(
            "{\"and\":[\"A\",\"B\",\"C\"]}",
            parse("A&B&C").unwrap().to_json_str()
        );
        assert_eq!(
            "{\"or\":[\"A\",{\"and\":[\"B\",\"C\"]}]}",
            parse("A|(B&C)").unwrap().to_json_str()
        );
    }

    #[test]
    fn a_redundant_scope_collapses_to_its_content() {
        assert_eq!(Ok(token("A")), parse("(((A)))"));
    }
}
