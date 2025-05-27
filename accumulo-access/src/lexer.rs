// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

use std::fmt::Display;
use std::iter::Peekable;
use std::str::Chars;
use thiserror::Error;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    #[allow(clippy::enum_variant_names)] AccessToken(String),
    OpenParen,
    CloseParen,
    And,
    Or,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
    Conjunction,
    Disjunction,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Token::AccessToken(token) => write!(f, "{:?}", token),
            Token::OpenParen => write!(f, "("),
            Token::CloseParen => write!(f, ")"),
            Token::And => write!(f, "&"),
            Token::Or => write!(f, "|"),
        }
    }
}

/// `Lexer` is a lexical analyzer (tokenizer) for authorization expressions.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    inner_peekable_iterator: Peekable<Chars<'a>>,
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
        let inner_peekable_iterator = input.chars().peekable();
        Lexer {
            inner_peekable_iterator,
            position: 0,
        }
    }

    fn read_char(&mut self) -> Option<char> {
        let c = self.inner_peekable_iterator.next();
        if c.is_some() {
            self.position += 1;
        }
        c
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.inner_peekable_iterator.peek()
    }
}

fn is_allowed_char_for_unquoted_access_token(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '_'
        || c == '-'
        || c == '.'
        || c == ':'
        || c == '/'
}

fn is_allowed_char_for_quoted_access_token(c: char) -> bool {
    // from SPECIFICATION.md:
    //
    // check that the character is in the valid ranges:
    // utf8-subset             = %x20-21 / %x23-5B / %x5D-7E / unicode-beyond-ascii ; utf8 minus '"' and '\'
    // unicode-beyond-ascii    = %x0080-D7FF / %xE000-10FFFF
    c.is_ascii_graphic()
        || c == ' '
        || (c as u32) >= 0x0080 && (c as u32) <= 0xD7FF
        || (c as u32) >= 0xE000 && (c as u32) <= 0x10FFFF
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.read_char()?;
        let r = match c {
            '(' => {
                //self.read_char();
                Ok(Token::OpenParen)
            }

            ')' => {
                //self.read_char();
                Ok(Token::CloseParen)
            }
            '&' => {
                //self.read_char();
                Ok(Token::And)
            }
            '|' => {
                //self.read_char();
                Ok(Token::Or)
            }
            '"' => {
                self.handle_quoted_access_token()
            }
            _ if is_allowed_char_for_unquoted_access_token(c) => {
                self.handle_unquoted_access_token(c)
            }
            _ => {
                //self.read_char();
                Err(LexerError::UnexpectedCharacter(c, self.position))
            }
        };
        Some(r)
    }
}

impl Lexer<'_> {
    fn handle_quoted_access_token(&mut self) -> Result<Token, LexerError> {
        let mut value = String::new();
        //self.read_char(); // discard the opening quote
        while let Some(c) = self.read_char() {
            if !is_allowed_char_for_quoted_access_token(c)
            {
                return Err(LexerError::UnexpectedCharacter(c, self.position));
            }
            match c {
                '\\' => {
                    if let Some(next_char) = self.read_char() {
                        if next_char == '"' || next_char == '\\' {
                            value.push(next_char);
                        } else {
                            return Err(LexerError::UnexpectedCharacter(next_char, self.position));
                        }
                    }
                }
                '"' => {
                    break;
                }
                _ => {
                    value.push(c);
                }
            }
        }
        Ok(Token::AccessToken(value))
    }

    fn handle_unquoted_access_token(&mut self, first_char: char) -> Result<Token, LexerError> {
        let mut value = String::new();
        value.push(first_char);
        while let Some(c) = self.peek_char() {
            if is_allowed_char_for_unquoted_access_token(*c) {
                let c = self.read_char().unwrap();
                value.push(c);
            } else {
                break;
            }
        }
        Ok(Token::AccessToken(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_valid() {
        let input =
            "label1&\"label 🕺\"|(\"hello \\\\ \\\"world\"|label4|(label5&label6)))";
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
    fn test_lexer_valid2() {
        let input = "\"abc!12\"&\"abc\\\\xyz\"&GHI";
        
        let lexer = Lexer::new(input);
        let tokens: Vec<Result<Token, LexerError>> = lexer.collect();
        
        assert_eq!(
            tokens,
            vec![
                Ok(Token::AccessToken("abc!12".to_string())),
                Ok(Token::And),
                Ok(Token::AccessToken("abc\\xyz".to_string())),
                Ok(Token::And),
                Ok(Token::AccessToken("GHI".to_string())),
            ]);
    }

    #[test]
    fn test_lexer_invalid() {
        let input = "label1 & [";
        let lexer = Lexer::new(input);
        let tokens: Vec<Result<Token, LexerError>> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(Token::AccessToken("label1".to_string())),
                Err(LexerError::UnexpectedCharacter(' ', 7)),
                Ok(Token::And),
                Err(LexerError::UnexpectedCharacter(' ', 9)),
                Err(LexerError::UnexpectedCharacter('[', 10)),
            ]
        );
    }
}
