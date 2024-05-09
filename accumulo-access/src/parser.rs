// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

use crate::lexer::{Lexer, Token};
use thiserror::Error;
use crate::authorization_expression::AuthorizationExpression;

/// `ParserError` is returned when the parser encounters an error.
#[derive(Error, Debug, PartialEq, Clone)]
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

#[derive(Debug)]
struct Scope {
    nodes: Vec<AuthorizationExpression>,
    labels: Vec<String>,
    operator: Option<Token>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            nodes: Vec::new(),
            labels: Vec::new(),
            operator: None,
        }
    }

    fn add_node(&mut self, token: AuthorizationExpression) {
        self.nodes.push(token);
    }

    fn add_label(&mut self, label: String) {
        self.labels.push(label);
    }

    fn set_operator(&mut self, operator: &Token) -> Result<(), ParserError> {
        match operator {
            Token::And => {
                if let Some(Token::Or) = self.operator {
                    return Err(ParserError::MixingOperators);
                }
            }
            Token::Or => {
                if let Some(Token::And) = self.operator {
                    return Err(ParserError::MixingOperators);
                }
            }
            _ => return Err(ParserError::UnexpectedToken(operator.clone())),
        }
        self.operator = Some(operator.clone());
        Ok(())
    }

    fn build(&mut self) -> Result<AuthorizationExpression, ParserError> {
        if self.labels.len() == 1 && self.nodes.is_empty() {
            return Ok(AuthorizationExpression::AccessToken(
                self.labels.pop().unwrap(),
            ));
        }
        // if it is a scope wrapping a single node, return the node
        if self.nodes.len() == 1 && self.labels.is_empty() {
            return Ok(self.nodes.pop().unwrap());
        }
        if self.operator.is_none() {
            return Err(ParserError::MissingOperator);
        }
        let operator = self.operator.take().unwrap();
        let mut nodes = Vec::with_capacity(self.labels.len() + self.nodes.len());

        while let Some(label) = self.labels.pop() {
            nodes.push(AuthorizationExpression::AccessToken(label));
        }

        while let Some(token) = self.nodes.pop() {
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
                Err(e) => return Err(ParserError::LexerError(e)),
            }
        }
        scope.build()
    }
}
