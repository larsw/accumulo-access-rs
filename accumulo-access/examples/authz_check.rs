// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

use std::collections::HashSet;
use std::env::args;

use accumulo_access::{Lexer, Parser};

fn main() {
    let mut args = args();
    args.next();
    let expression = args.next().expect("Missing expression");
    let tokens = args.next().expect("Missing tokens");
    let json = match args.next() {
        Some(last) => {
            // if last == '--json'
            if last == "--json" {
                true
            } else {
                false
            }
        }
        None => false,
    };
    let authorized_tokens: HashSet<String> =
        tokens.split(',').map(|token| token.to_string()).collect();

    let lexer: Lexer<'_> = Lexer::new(&expression);
    let mut parser = Parser::new(lexer);

    match parser.parse() {
        Ok(auth_expr) => {
            let result = auth_expr.evaluate(&authorized_tokens);

            if json {
                println!("{}", auth_expr.to_json_str());
            } else {
                println!("{}", result);
            }
            if result {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
        Err(e) => {
            println!("{}", e);
            std::process::exit(-1);
        }
    }
}
