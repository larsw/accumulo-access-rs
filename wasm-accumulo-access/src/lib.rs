// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

use js_sys::Array;
use wasm_bindgen::prelude::*;

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
/// const expression = 'label1|label5';
/// const tokens = ['label1', 'label5', 'label 🕺'];
/// const result = checkAuthorization(expression, tokens);
/// console.log(result);
// Errors are returned rather than thrown with `throw_str`, which would skip destructors and leak
// memory on every invalid expression; wasm-bindgen still throws them in JS.
#[wasm_bindgen(js_name = checkAuthorization, skip_typescript)]
pub fn check_authorization(expression: &str, tokens: &Array) -> Result<bool, JsValue> {
    let tokens = tokens
        .iter()
        .map(|token| {
            token
                .as_string()
                .ok_or_else(|| js_sys::TypeError::new("tokens must be strings").into())
        })
        .collect::<Result<Vec<String>, JsValue>>()?;
    accumulo_access::check_authorization(expression, &tokens).map_err(to_js_error)
}

#[wasm_bindgen(js_name = toExpressionTree, skip_typescript)]
pub fn to_expression_tree(expression: &str) -> Result<JsValue, JsValue> {
    js_sys::JSON::parse(&expression_to_json_string(expression)?)
}

#[wasm_bindgen(js_name = toExpressionTreeJson, skip_typescript)]
pub fn to_expression_tree_json(expression: &str) -> Result<JsValue, JsValue> {
    expression_to_json_string(expression).map(JsValue::from)
}

/// Parses the expression and returns its expression tree as a JSON string.
fn expression_to_json_string(expression: &str) -> Result<String, JsValue> {
    accumulo_access::expression_to_json_string(expression).map_err(to_js_error)
}

fn to_js_error(error: accumulo_access::ParserError) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
