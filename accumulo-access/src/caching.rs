// Copyright 2024 Lars Wilhelmsen <sral-backwards@sral.org>. All rights reserved.
// Use of this source code is governed by the MIT or Apache-2.0 license that can be found in the LICENSE_MIT or LICENSE_APACHE files.

use cached::{proc_macro::cached, Cached, LruCache};

const DEFAULT_CACHE_SIZE: usize = 20000;

fn get_cache_size() -> usize {
    let size = std::env::var("ACCUMULO_ACCESS_CACHE_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_CACHE_SIZE);
    // `LruCache::new` panics on a zero capacity, and this runs lazily on the
    // first lookup -- inside a Postgres backend, in the case of the extension.
    size.max(1)
}

/// Builds the cache key for an (expression, tokens) pair.
///
/// Length-prefixed rather than concatenated: `("a", "bc")` and `("ab", "c")`
/// both concatenate to `"abc"`, which would let one pair's cached decision
/// answer for the other.
fn cache_key(expression: &str, tokens: &str) -> String {
    format!("{}:{}:{}", expression.len(), expression, tokens)
}

// A `Result`-returning body puts the macro on its fallible path: only the `Ok`
// value is stored, so a parse error is returned to every caller and never cached.
#[cached(
    ty = "LruCache<String, bool>",
    create = "{ LruCache::new(get_cache_size()) }",
    convert = r##"{ cache_key(&expression, &tokens) }"##
)]
pub fn check_authorization_csv(
    expression: String,
    tokens: String,
) -> Result<bool, super::ParserError> {
    super::check_authorization_csv(expression, tokens)
}

pub fn clear_authz_cache() -> Result<(), String> {
    let mut cache = crate::caching::CHECK_AUTHORIZATION_CSV.write();
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
        Self { hits, misses, size }
    }
}

pub fn authz_cache_stats() -> Result<AuthzCacheStats, String> {
    let cache = crate::caching::CHECK_AUTHORIZATION_CSV.read();

    Ok(AuthzCacheStats::new(
        cache.cache_hits().unwrap_or(0),
        cache.cache_misses().unwrap_or(0),
        cache.cache_size(),
    ))
}

#[cfg(test)]
mod tests {
    use super::cache_key;

    #[test]
    fn cache_key_separates_the_expression_from_the_tokens() {
        // Plain concatenation would give both of these the key "abc", letting
        // one pair's cached authorization decision answer for the other.
        assert_ne!(cache_key("a", "bc"), cache_key("ab", "c"));
        assert_ne!(cache_key("", "ab"), cache_key("ab", ""));
    }

    #[test]
    fn cache_key_is_stable_for_the_same_pair() {
        assert_eq!(cache_key("label1&label2", "label1"), cache_key("label1&label2", "label1"));
    }
}
