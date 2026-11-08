// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_accumulo_access::check_authorization;
use wasm_accumulo_access::to_expression_tree;
use wasm_accumulo_access::to_expression_tree_json;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test1() {
    let tokens: &js_sys::Array = &[String::from("label1"), String::from("label5")]
        .iter()
        .cloned()
        .map(wasm_bindgen::JsValue::from)
        .collect();
    let result = check_authorization("label1|label5", tokens);
    assert_eq!(result, Ok(true));
}

const EXPRESSION: &str = "ALL_ACCESS|(SECRET&NTK&(COI1|COI2))";

#[wasm_bindgen_test]
fn test2() {
    let json = to_expression_tree_json(EXPRESSION).unwrap();
    assert_eq!(
        json.as_string(),
        accumulo_access::expression_to_json_string(EXPRESSION).ok()
    );
    web_sys::console::log_1(&json);
}

#[wasm_bindgen_test]
fn test3() {
    // test to_expression_tree()
    let ast = to_expression_tree(EXPRESSION).unwrap();
    assert_eq!(
        String::from(js_sys::JSON::stringify(&ast).unwrap()),
        accumulo_access::expression_to_json_string(EXPRESSION).unwrap()
    );
    web_sys::console::dir_1(&ast);
}

#[wasm_bindgen_test]
fn test_errors_are_returned_not_trapped() {
    let not_a_string = js_sys::Array::of1(&wasm_bindgen::JsValue::from_f64(1.0));
    assert!(check_authorization("A", &not_a_string).is_err());
    assert!(check_authorization("A&", &js_sys::Array::new()).is_err());
    assert!(to_expression_tree("(").is_err());
    assert!(to_expression_tree_json("A)").is_err());
}
