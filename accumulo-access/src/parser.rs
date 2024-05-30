// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

use crate::lexer::{Lexer, Operator, Token};
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
    /// The parser encountered a lexer error.
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
    access_tokens: Vec<String>,
    operator: Option<Operator>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            nodes: Vec::new(),
            access_tokens: Vec::new(),
            operator: None,
        }
    }

    fn append_node(&mut self, token: AuthorizationExpression) {
        self.nodes.push(token);
    }

    fn append_access_token(&mut self, label: String) {
        self.access_tokens.push(label);
    }

    fn disjunction(&mut self) -> Result<(), ParserError> {
        self.set_operator(&Operator::Disjunction)
    }

    fn conjunction(&mut self) -> Result<(), ParserError> {
        self.set_operator(&Operator::Conjunction)
    }

    fn set_operator(&mut self, operator: &Operator) -> Result<(), ParserError> {
        match operator {
            Operator::Conjunction => {
                if let Some(Operator::Disjunction) = self.operator {
                    return Err(ParserError::MixingOperators);
                }
            }
            Operator::Disjunction => {
                if let Some(Operator::Conjunction) = self.operator {
                    return Err(ParserError::MixingOperators);
                }
            }
        }
        self.operator = Some(operator.clone());
        Ok(())
    }

    fn build(&mut self) -> Result<AuthorizationExpression, ParserError> {
       if self.access_tokens.is_empty() && self.nodes.is_empty() {
           return Ok(AuthorizationExpression::Nil)
       }
       
        if self.access_tokens.len() == 1 && self.nodes.is_empty() {
            return Ok(AuthorizationExpression::AccessToken(
                self.access_tokens.pop().unwrap(),
            ));
        }
        // if it is a scope wrapping a single node, return the node
        if self.nodes.len() == 1 && self.access_tokens.is_empty() {
            return Ok(self.nodes.pop().unwrap());
        }
        if self.operator.is_none() {
            return Err(ParserError::MissingOperator);
        }
        let operator = self.operator.take().unwrap();
        let mut nodes = Vec::with_capacity(self.access_tokens.len() + self.nodes.len());

        while let Some(label) = self.access_tokens.pop() {
            nodes.push(AuthorizationExpression::AccessToken(label));
        }

        while let Some(token) = self.nodes.pop() {
            nodes.push(token);
        }
        match operator {
            Operator::Conjunction => Ok(AuthorizationExpression::ConjunctionOf(nodes)),
            Operator::Disjunction => Ok(AuthorizationExpression::DisjunctionOf(nodes))
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
        let mut scope = Scope::new();
        while let Some(result) = self.lexer.next() {
            match result {
                Ok(token) => {
                    match token {
                        Token::AccessToken(value) => scope.append_access_token(value),
                        Token::OpenParen => {
                            let node = self.parse()?;
                            scope.append_node(node.clone()); // The clone here is apparently important.
                        }
                        Token::And => scope.conjunction()?,
                        Token::Or => scope.disjunction()?,
                        Token::CloseParen => return scope.build(),
                    }
                }
                Err(e) => {
                    return Err(ParserError::LexerError(e));  
                } 
            }
        }
        scope.build()
    }
}
