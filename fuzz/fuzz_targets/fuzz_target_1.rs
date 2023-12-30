// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

#![no_main]
//#[macro_use] extern crate libfuzzer_sys;
extern crate accumulo_access;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let expression = std::str::from_utf8(data);
    if expression.is_err() {
        return;
    }
    let lexer = accumulo_access::Lexer::new(&expression.unwrap());
    let mut parser = accumulo_access::Parser::new(lexer);
    let _ = parser.parse();
});
