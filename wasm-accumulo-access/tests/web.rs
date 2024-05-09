// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;
use wasm_accumulo_access::check_authorization;
use wasm_accumulo_access::to_expression_tree_json;
use wasm_accumulo_access::to_expression_tree;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test1() {
    let tokens : &js_sys::Array = &[String::from("label1"), String::from("label5")]
        .iter()
        .cloned()
        .map(wasm_bindgen::JsValue::from)
        .collect();
    let result = check_authorization("label1 | label5", &tokens);
    assert_eq!(result, true);
}

#[wasm_bindgen_test]
fn test2() {
    let ast = to_expression_tree_json("ALL_ACCESS | (SECRET & NTK & (COI1 | COI2))");
    match ast {
        Ok(val) => {
            web_sys::console::log_1(&val);
        }
        Err(err) => {
            web_sys::console::error_1(&err);
        }
    }
}

#[wasm_bindgen_test]
fn test3() {
    // test to_expression_tree()
    let ast = to_expression_tree("ALL_ACCESS | (SECRET & NTK & (COI1 | COI2))");

    match ast {
        Ok(val) => {
            web_sys::console::dir_1(&val);
        }
        Err(err) => {
            web_sys::console::error_1(&err);
        }
    }
}
