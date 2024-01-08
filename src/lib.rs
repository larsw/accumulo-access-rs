use std::collections::HashSet;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Debug, PartialEq)]
pub enum AuthorizationExpression {
    And(Vec<AuthorizationExpression>),
    Or(Vec<AuthorizationExpression>),
    AccessToken(String),
}

impl AuthorizationExpression {
    pub fn and(expressions: Vec<AuthorizationExpression>) -> AuthorizationExpression {
        AuthorizationExpression::And(expressions)
    }

    pub fn or(expressions: Vec<AuthorizationExpression>) -> AuthorizationExpression {
        AuthorizationExpression::Or(expressions)
    }

    pub fn access_token(token: String) -> AuthorizationExpression {
        AuthorizationExpression::AccessToken(token)
    }

    pub fn evaluate(
        expr: &AuthorizationExpression,
        authorizations: &Authorizations,
    ) -> Result<bool, String> {
        match expr {
            AuthorizationExpression::And(expressions) => Ok(expressions
                .iter()
                .all(|expr| AuthorizationExpression::evaluate(expr, authorizations).unwrap())),
            AuthorizationExpression::Or(expressions) => Ok(expressions
                .iter()
                .any(|expr| AuthorizationExpression::evaluate(expr, authorizations).unwrap())),
            AuthorizationExpression::AccessToken(value) => {
                Ok(authorizations.is_authorized_for(value))
            }
        }
    }
}

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct AccessExpressionParser;

// a function that parses a string With Rule::access_expression and transforms it into a tree of AuthorizationExpression.
pub fn parse_expr(expr: &str) -> Result<AuthorizationExpression, String> {
    let mut pairs = AccessExpressionParser::parse(Rule::access_expression, expr)
        .map_err(|e| format!("Error parsing expression: {}", e))?;

    let access_expression = pairs.next().unwrap();

    println!("Parsed expression: {:?}", access_expression);

    fn parse_(pair: Pair<Rule>) -> AuthorizationExpression {
        match pair.as_rule() {
            Rule::access_expression => {
                parse_(pair.into_inner().next().unwrap()) 
            },
            Rule::and_expression => {
                let mut expressions = vec![];
                for inner in pair.into_inner() {
                    expressions.push(parse_(inner));
                }
                AuthorizationExpression::And(expressions)
            }
            Rule::or_expression => {
                let mut expressions = vec![];
                for inner in pair.into_inner() {
                    expressions.push(parse_(inner));
                }
                AuthorizationExpression::Or(expressions)
            }
            Rule::access_token => {
                AuthorizationExpression::AccessToken(pair.as_str().to_string())
            },
            _ => unreachable!(),
        }
    }
    Ok(parse_(access_expression))
}

// pub fn parse_expr(expr: &str) -> Result<AuthorizationExpression, String> {
//     let mut pairs = AccessExpressionParser::parse(Rule::access_expression, expr)
//         .map_err(|e| format!("Error parsing expression: {}", e))?;

//     // print pairs nicely:
//     for pair in pairs.clone() {
//         display_pair(pair, 0);
//         println!();
//     }
//     let access_expression = pairs.next().unwrap();

//     println!("Parsed expression: {:?}", access_expression);

//     fn parse_(pair: Pair<Rule>) -> AuthorizationExpression {
//         match pair.as_rule() {
//             Rule::access_expression => parse_(pair.into_inner().next().unwrap()),
//             Rule::and_expression => {
//                 let mut expressions = vec![];
//                 for inner in pair.into_inner() {
//                     expressions.push(parse_(inner));
//                 }
//                 AuthorizationExpression::And(expressions)
//             }
//             Rule::or_expression => {
//                 let mut expressions = vec![];
//                 for inner in pair.into_inner() {
//                     expressions.push(parse_(inner));
//                 }
//                 AuthorizationExpression::Or(expressions)
//             }
//             Rule::access_token => AuthorizationExpression::AccessToken(pair.as_str().to_string()),
//             _ => unreachable!(),
//         }
//     }
//     Ok(parse_(access_expression))
// }

pub struct Authorizations {
    values: HashSet<String>,
}

impl Authorizations {
    pub fn new(values: &[&str]) -> Authorizations {
        Authorizations {
            values: values.iter().map(|v| v.to_string()).collect(),
        }
    }

    pub fn is_authorized_for(&self, value: &str) -> bool {
        self.values.contains(value)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn parse_and_evaluate() -> Result<(), String> {
        let expression = "label1 & label2 & label3";

        let authorizations = Authorizations::new(&["label1", "label2", "label3"]);

        let parsed_expr = parse_expr(expression)?;
        let result = AuthorizationExpression::evaluate(&parsed_expr, &authorizations)?;
        assert_eq!(result, true);
        println!("Evaluation result: {}", result);
        Ok(())
    }

    #[test]
    fn parse_and_evaluate_false() -> Result<(), String> {
        let expression = "label1 & label2 & label3";

        let authorizations = Authorizations::new(&["label2", "label3"]);

        let parsed_expr = parse_expr(expression)?;
        let result = AuthorizationExpression::evaluate(&parsed_expr, &authorizations)?;
        assert_eq!(result, false);
        println!("Evaluation result: {}", result);
        Ok(())
    }

    #[rstest]
    #[case("label1 & label2 & label3", &["label2", "label3"], false)]
    #[case("label1 & label2 & label3", &["label2", "label3", "label1"], true)]
    #[case("label1 & (label2 | label3)", &["label1", "label3"], true)]
    #[case("label1 & (label2 | label3)", &["label1"], false)]
    #[case("label1 | ((label2))", &["label2"], true)]
    fn parse_and_evaluate_parameterized(
        #[case] expression: &str,
        #[case] labels: &[&str],
        #[case] expected: bool,
    ) -> Result<(), String> {
        let authorizations = Authorizations::new(labels);
        let parsed_expr = parse_expr(expression)?;
        let result = AuthorizationExpression::evaluate(&parsed_expr, &authorizations)?;
        assert_eq!(result, expected);
        Ok(())
    }
}
