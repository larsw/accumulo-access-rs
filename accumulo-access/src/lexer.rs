// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    #[allow(clippy::enum_variant_names)] AccessToken(String),
    OpenParen,
    CloseParen,
    And,
    Or,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Token::AccessToken(token) => write!(f, "{:?}", token),
            Token::OpenParen => write!(f, "("),
            Token::CloseParen => write!(f, ")"),
            Token::And => write!(f, " & "),
            Token::Or => write!(f, " | "),
        }
    }
}

/// `Lexer` is a lexical analyzer (tokenizer) for authorization expressions.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    input: std::str::Chars<'a>,
    next_char: Option<char>,
    position: usize,
}

#[derive(Error, Debug, PartialEq, Clone)]
pub enum LexerError {
    UnexpectedCharacter(char, usize),
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LexerError::UnexpectedCharacter(c, position) => {
                write!(f, "Unexpected character '{}' at position {}", c, position)
            }
        }
    }
}

impl<'a> Lexer<'a> {
    /// Creates a new `Lexer` instance.
    ///
    /// # Arguments
    ///
    /// * `input` - The authorization expression to tokenize.
    pub fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let next_char = chars.next();
        Lexer {
            input: chars,
            next_char,
            position: 0,
        }
    }

    fn read_char(&mut self) -> Option<char> {
        let current_char = self.next_char;
        self.next_char = self.input.next();
        self.position += 1;
        current_char
    }

    fn peek_char(&self) -> Option<char> {
        self.next_char
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        fn is_allowed_character(c: char) -> bool {
            c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ':'
        }

        while let Some(c) = self.peek_char() {
            match c {
                ' ' | '\n' | '\t' => {
                    self.read_char();
                }
                '(' => {
                    self.read_char();
                    return Some(Ok(Token::OpenParen));
                }
                ')' => {
                    self.read_char();
                    return Some(Ok(Token::CloseParen));
                }
                '&' => {
                    self.read_char();
                    return Some(Ok(Token::And));
                }
                '|' => {
                    self.read_char();
                    return Some(Ok(Token::Or));
                }
                '"' => {
                    self.read_char();
                    let mut string = String::new();
                    while let Some(c) = self.peek_char() {
                        match c {
                            '\\' => {
                                self.read_char();
                                if let Some(next_char) = self.peek_char() {
                                    if next_char == '"' || next_char == '\\' {
                                        string.push(self.read_char().unwrap());
                                    }
                                }
                            }
                            '"' => {
                                break;
                            }
                            _ => {
                                string.push(c);
                                self.read_char();
                            }
                        }
                    }
                    self.read_char();
                    return Some(Ok(Token::AccessToken(string)));
                }
                _ if is_allowed_character(c) => {
                    let mut id = String::new();
                    while let Some(c) = self.peek_char() {
                        if is_allowed_character(c) {
                            id.push(c);
                            self.read_char();
                        } else {
                            break;
                        }
                    }
                    return Some(Ok(Token::AccessToken(id)));
                }
                _ => {
                    self.read_char();
                    return Some(Err(LexerError::UnexpectedCharacter(c, self.position)))
                },
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_valid() {
        let input =
            "label1 & \"label 🕺\" | (\"hello \\\\ \\\"world\" | label4 | (label5 & label6)))";
        let lexer = Lexer::new(input);
        let tokens: Vec<Result<Token, LexerError>> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(Token::AccessToken("label1".to_string())),
                Ok(Token::And),
                Ok(Token::AccessToken("label 🕺".to_string())),
                Ok(Token::Or),
                Ok(Token::OpenParen),
                Ok(Token::AccessToken("hello \\ \"world".to_string())),
                Ok(Token::Or),
                Ok(Token::AccessToken("label4".to_string())),
                Ok(Token::Or),
                Ok(Token::OpenParen),
                Ok(Token::AccessToken("label5".to_string())),
                Ok(Token::And),
                Ok(Token::AccessToken("label6".to_string())),
                Ok(Token::CloseParen),
                Ok(Token::CloseParen),
                Ok(Token::CloseParen),
            ]
        );
    }

    #[test]
    fn test_lexer_invalid() {
            let input = "label1 & [";
        let lexer = Lexer::new(input);
        let tokens: Vec<Result<Token, LexerError>> = lexer.collect();
        assert_ne!(
            tokens,
            vec![
                Ok(Token::AccessToken("label1".to_string())),
                Ok(Token::And),
                Err(LexerError::UnexpectedCharacter('[', 9)),
            ]
        ); }
}
