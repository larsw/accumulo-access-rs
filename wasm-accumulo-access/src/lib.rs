// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

use accumulo_access::{Lexer, Parser};
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_str;

use js_sys::{Array, Object, Reflect};
use accumulo_access::authorization_expression::AuthorizationExpression;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
/// Parses and evaluate the given expression against the given access tokens.
/// Returns true if the expression is valid and the tokens are authorized.
export const checkAuthorization = (expression: string, tokens: string[]): boolean;
export const toExpressionTree = (expression: string): Object;
export const toExpressionTreeJson = (expression: string): string;
"#;

/// Parses and evaluate the given expression against the given access tokens.
/// Returns true if the expression is valid and the tokens are authorized.
///
/// # Arguments
/// expression - The expression to parse and evaluate.
/// tokens - The access tokens to check.
///
/// # Examples
/// ```ts
/// import { checkAuthorization } from 'accumulo-access';
///
/// const expression = 'label1 | label5';
/// const tokens = ['label1', 'label5', 'label 🕺'];
/// const result = checkAuthorization(expression, tokens);
/// console.log(result);
#[wasm_bindgen(js_name = checkAuthorization, skip_typescript)]
pub fn check_authorization(expression: &str, tokens: &Array) -> bool {
    let lexer: Lexer<'_> = Lexer::new(expression);
    let mut parser = Parser::new(lexer);

    match parser.parse() {
        Ok(auth_expr) => {
            let authorized_labels: HashSet<String> =
                tokens.iter().map(|s| s.as_string().unwrap()).collect();
            auth_expr.evaluate(&authorized_labels)
        }
        Err(e) => throw_str(e.to_string().as_str()),
    }
}

#[wasm_bindgen(js_name = toExpressionTree, skip_typescript)]
pub fn to_expression_tree(expression: &str) -> Result<JsValue, JsValue> {
    let lexer: Lexer<'_> = Lexer::new(expression);
    let mut parser = Parser::new(lexer);

    match parser.parse() {
        Ok(auth_expr) => {
            to_expression_tree_inner(&auth_expr)
        }
        Err(e) => {
            Err(js_sys::Error::new(e.to_string().as_str()).into())
        }
    }
}

fn to_expression_tree_inner(expression: &AuthorizationExpression) -> Result<JsValue, JsValue> {
    match expression {
        AuthorizationExpression::ConjunctionOf(nodes) => {
            let labels = Array::new();
            for node in nodes {
                labels.push(&to_expression_tree_inner(node).unwrap());

            }
            let and_node = Object::new();
            Reflect::set(&and_node, &JsValue::from("and"), &labels).unwrap();
            Ok(JsValue::from(and_node))
        }
        AuthorizationExpression::DisjunctionOf(nodes) => {
            let labels = Array::new();
            for node in nodes {
                labels.push(&to_expression_tree_inner(node).unwrap());
            }
            let and_node = Object::new();
            Reflect::set(&and_node, &JsValue::from("or"), &labels).unwrap();
            Ok(JsValue::from(and_node))
        }
        AuthorizationExpression::AccessToken(token) => Ok(JsValue::from(token)),
    }
}

#[wasm_bindgen(js_name = toExpressionTreeJson, skip_typescript)]
pub fn to_expression_tree_json(expression: &str) -> Result<JsValue, JsValue> {
    let lexer: Lexer<'_> = Lexer::new(expression);
    let mut parser = Parser::new(lexer);

    match parser.parse() {
        Ok(auth_expr) => {
            Ok(JsValue::from_str(auth_expr.to_json_str().as_str()))
        }
        Err(e) => {
            Err(js_sys::Error::new(e.to_string().as_str()).into())
        }
    }
}
