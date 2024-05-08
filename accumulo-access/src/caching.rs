// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

use cached::{proc_macro::cached, Cached, SizedCache};

fn get_cache_size() -> usize {
    std::env::var("ACCUMULO_ACCESS_CACHE_SIZE")
        .unwrap_or_else(|_| String::from("20000"))
        .parse::<usize>()
        .unwrap_or(20000)
}

#[cached(
type = "SizedCache<String, Result<bool, super::ParserError>>",
create = "{ SizedCache::with_size(get_cache_size()) }",
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
