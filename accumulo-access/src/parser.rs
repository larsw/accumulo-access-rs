// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

use std::collections::HashSet;
use crate::lexer::{Lexer, Token};
use crate::ParserError::LexerError;

/// `ParserError` is returned when the parser encounters an error.
#[derive(Debug)]
pub enum ParserError {
    /// The scope (top-level or set of parentheses) is empty.
    EmptyScope,
    /// The scope is missing an operator ('&' or '|').
    MissingOperator,
    /// The parser encountered an unexpected token.
    UnexpectedToken(Token),
    /// The parser encountered a mix of operators ('&' and '|').
    MixingOperators,
    LexerError(crate::lexer::LexerError),
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::EmptyScope => write!(f, "Empty scope"),
            ParserError::MissingOperator => write!(f, "Missing operator"),
            ParserError::UnexpectedToken(token) => write!(f, "Unexpected token: {}", token),
            ParserError::MixingOperators => write!(f, "Mixing operators"),
            ParserError::LexerError(e) => write!(f, "{}", e),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AuthorizationExpression {
    And(Vec<AuthorizationExpression>),
    Or(Vec<AuthorizationExpression>),
    AccessToken(String),
}

impl AuthorizationExpression {
    pub fn evaluate(&self, authorizations: &HashSet<String>) -> bool {
        match self {
            AuthorizationExpression::And(nodes) =>
                nodes.iter().all(|node| node.evaluate(authorizations)),

            AuthorizationExpression::Or(nodes) =>
                nodes.iter().any(|node| node.evaluate(authorizations)),

            AuthorizationExpression::AccessToken(token) => authorizations.contains(token),
        }
    }

    pub fn to_json_str(&self) -> String {
        match self {
            AuthorizationExpression::And(nodes) => {
                let mut json = String::from("{\"and\": [");
                for node in nodes {
                    json.push_str(&node.to_json_str());
                    json.push(',');
                }
                json.pop();
                json.push(']');
                json.push('}');
                json
            }
            AuthorizationExpression::Or(nodes) => {
                let mut json = String::from("{\"or\": [");
                for node in nodes {
                    json.push_str(&node.to_json_str());
                    json.push(',');
                }
                json.pop();
                json.push(']');
                json.push('}');
                json
            }
            AuthorizationExpression::AccessToken(token) => format!("\"{}\"", token),
        }
    }
}

#[derive(Debug)]
struct Scope {
    tokens: Vec<AuthorizationExpression>,
    labels: Vec<String>,
    current_operator: Option<Token>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            tokens: Vec::new(),
            labels: Vec::new(),
            current_operator: None,
        }
    }

    fn add_node(&mut self, token: AuthorizationExpression) {
        self.tokens.push(token);
    }

    fn add_label(&mut self, label: String) {
        self.labels.push(label);
    }

    fn set_operator(&mut self, operator: &Token) -> Result<(), ParserError> {
        match operator {
            Token::And => {
                if let Some(Token::Or) = self.current_operator {
                    return Err(ParserError::MixingOperators);
                }
            }
            Token::Or => {
                if let Some(Token::And) = self.current_operator {
                    return Err(ParserError::MixingOperators);
                }
            }
            _ => return Err(ParserError::UnexpectedToken(operator.clone())),
        }
        self.current_operator = Some(operator.clone());
        Ok(())
    }

    fn build(&mut self) -> Result<AuthorizationExpression, ParserError> {
        if self.labels.is_empty() {
            return Err(ParserError::EmptyScope);
        }
        if self.labels.len() == 1 && self.tokens.is_empty() {
            return Ok(AuthorizationExpression::AccessToken(
                self.labels.pop().unwrap(),
            ));
        }
        if self.current_operator.is_none() {
            return Err(ParserError::MissingOperator);
        }
        let operator = self.current_operator.take().unwrap();
        let mut nodes = Vec::with_capacity(self.labels.len() + self.tokens.len());

        while let Some(label) = self.labels.pop() {
            nodes.push(AuthorizationExpression::AccessToken(label));
        }

        while let Some(token) = self.tokens.pop() {
            nodes.push(token);
        }
        match operator {
            Token::And => Ok(AuthorizationExpression::And(nodes)),
            Token::Or => Ok(AuthorizationExpression::Or(nodes)),
            _ => Err(ParserError::UnexpectedToken(operator)),
        }
    }
}

/// `Parser` is used to parse an expression and return an `AuthorizationExpression`-based tree.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    /// Creates a new `Parser` instance.
    ///
    /// # Arguments
    ///
    /// * `lexer` - The `Lexer` instance to use for tokenization.
    pub fn new(lexer: Lexer<'a>) -> Self {
        Parser { lexer }
    }

    /// Parse the input string and return an AuthorizationExpression.
    /// If the input string is invalid, a ParserError is returned.
    ///
    /// # Example
    /// ```
    ///  use std::collections::HashSet;
    ///  use accumulo_access::{Lexer, Parser};
    ///  let input = "label1 & label5 & (label3 | label8 | \"label 🕺\")";
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
        let mut scope = Scope::new();
        while let Some(result) = self.lexer.next() {
            match result {
                Ok(token) => {
                    match token {
                        Token::AccessToken(value) => scope.add_label(value),
                        Token::OpenParen => {
                            let node = self.parse()?;
                            scope.add_node(node.clone()); // The clone here is apparently important.
                        }
                        Token::And => scope.set_operator(&Token::And)?,
                        Token::Or => scope.set_operator(&Token::Or)?,
                        Token::CloseParen => return scope.build(),
                    }
                }
                Err(e) => return Err(LexerError(e)),
            }
        }
        scope.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_check_authorization() {
        let input = "label1 & label5 & (label3 | label8 | \"label 🕺\")";
        let lexer: Lexer<'_> = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().unwrap();
        let mut authorized_tokens = HashSet::new();
        authorized_tokens.insert(String::from("label1"));
        authorized_tokens.insert(String::from("label5"));
        authorized_tokens.insert(String::from("label 🕺"));

        assert_eq!(ast.evaluate(&authorized_tokens), true);
    }

    #[test]
    fn test_check_authorization2() {
        let input = "label1 & label2 & (label4 | label5)";
        let lexer: Lexer<'_> = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().unwrap();
        let mut authorized_tokens = HashSet::new();
        authorized_tokens.insert(String::from("label1"));
        authorized_tokens.insert(String::from("label2"));

        println!("{:?}", ast);

        assert_eq!(ast.evaluate(&authorized_tokens), false);
    }

    #[test]
    fn test_check_authorization3() {
        let input = "label1 & (label3 | label4)";
        let lexer: Lexer<'_> = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().unwrap();
        let mut authorized_tokens = HashSet::new();
        authorized_tokens.insert(String::from("label1"));
        authorized_tokens.insert(String::from("label2"));

        assert_eq!(ast.evaluate(&authorized_tokens), false);
    }
}
