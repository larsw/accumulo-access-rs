// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

use accumulo_access::{Lexer, Parser};
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::throw_str;

use js_sys::Array;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
/// Parses and evaluate the given expression against the given access tokens.
/// Returns true if the expression is valid and the tokens are authorized.
export const checkAuthorization = (expression: string, tokens: string[]): boolean;
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
    let lexer: Lexer<'_> = Lexer::new(&expression);
    let mut parser = Parser::new(lexer);

    match parser.parse() {
        Ok(auth_expr) => {
            let authorized_labels: HashSet<String> =
                tokens.iter().map(|s| s.as_string().unwrap()).collect();
            let result = auth_expr.evaluate(&authorized_labels);
            result
        }
        Err(e) => throw_str(e.to_string().as_str()),
    }
}
