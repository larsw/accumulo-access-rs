// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE-MIT or LICENSE-APACHE files.

use cached::{proc_macro::cached, Cached, SizedCache};

#[cached(
type = "SizedCache<String, Result<bool, super::ParserError>>",
create = "{ SizedCache::with_size(20000) }",
convert = r##"{ format!("{}{}", expression, tokens) }"##
)]
pub fn check_authorization_csv(
    expression: String,
    tokens: String,
) -> Result<bool, super::ParserError> {
    super::check_authorization_csv(expression, tokens)
}

pub fn clear_authz_cache() -> Result<(), String> {
    let mut cache = crate::caching::CHECK_AUTHORIZATION_CSV
        .lock()
        .map_err(|e| format!("Failed to lock cache: {}", e))?;
    cache.cache_clear();
    Ok(())
}

pub struct AuthzCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
}

impl AuthzCacheStats {
    pub fn new(hits: u64, misses: u64, size: usize) -> Self {
        Self {
            hits,
            misses,
            size,
        }
    }
}

pub fn authz_cache_stats() -> Result<AuthzCacheStats, String> {
    let cache = crate::caching::CHECK_AUTHORIZATION_CSV
        .lock()
        .map_err(|e| format!("Failed to lock cache: {}", e))?;

    Ok(AuthzCacheStats::new(cache.cache_hits().unwrap(), cache.cache_misses().unwrap(), cache.cache_size()))
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use rstest::rstest;

//     #[rstest]
//     #[case("label1", "label1", true)]
//     #[case("label1|label2", "label1", true)]
//     #[case("label1&label2", "label1", false)]
//     #[case("label1&label2", "label1,label2", true)]
//     #[case("label1&(label2 | label3)", "label1", false)]
//     #[case("label1&(label2 | label3)", "label1,label3", true)]
//     #[case("label1&(label2 | label3)", "label1,label2", true)]
//     #[case("(label2 | label3)", "label1", false)]
//     #[case("(label2 | label3)", "label2", true)]
//     #[case("(label2 & label3)", "label2", false)]
//     #[case("((label2 | label3))", "label2", true)]
//     #[case("((label2 & label3))", "label2", false)]
//     #[case("(((((label2 & label3)))))", "label2", false)]
//     fn test_check_authorization(
//         #[case] expr: impl AsRef<str>,
//         #[case] authorized_tokens: impl AsRef<str>,
//         #[case] expected: bool,
//     ) {
//         let result =
//             check_authorization_csv(expr.as_ref(), authorized_tokens.as_ref()).unwrap();
//         assert_eq!(result, expected);
//     }
// }