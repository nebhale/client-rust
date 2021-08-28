/*
 * Copyright 2021 the original author or authors.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref VALID_SECRET_KEY: Regex = Regex::new(r"^[A-Za-z0-9\-_.]+$").unwrap();
}

/// Tests whether a `str` is a valid
/// [Kubernetes Secret key](https://kubernetes.io/docs/concepts/configuration/secret/#overview-of-secrets).
///
/// * `key` - the key to check
///
/// returns `true` if the `str` is a valid Kubernetes Secret key, otherwise `false`
pub fn is_valid_secret_key(key: &str) -> bool {
    return VALID_SECRET_KEY.is_match(key);
}

#[cfg(test)]
mod tests {
    use crate::secret::is_valid_secret_key;

    #[test]
    fn is_valid_secret_key_valid() {
        let valid = [
            "alpha",
            "BRAVO",
            "Charlie",
            "delta01",
            "echo-foxtrot",
            "golf_hotel",
            "india.juliet",
            ".kilo",
        ];

        for v in valid {
            assert!(is_valid_secret_key(v));
        }
    }

    #[test]
    fn is_valid_secret_key_invalid() {
        let valid = [
            "lima^mike",
        ];

        for v in valid {
            assert!(!is_valid_secret_key(v));
        }
    }
}
