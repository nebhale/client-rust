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

use std::collections::HashMap;
use std::env;
use std::path::Path;

use crate::binding::{Binding, CacheBinding, HashMapBinding};

pub const SERVICE_BINDING_ROOT: &str = "SERVICE_BINDING_ROOT";

/// Wraps each `Binding` in a `CacheBinding`.
///
/// * `bindings` - the bindings to wrap
/// returns the wrapped `Binding`s
pub fn cached<'a>(bindings: Vec<impl Binding + 'a>) -> Vec<impl Binding + 'a> {
    return bindings.into_iter()
        .map(|v| CacheBinding::new(v))
        .collect();
}

/// Creates a new collection of `Binding`s using the specified root.  If the directory does not exist, an empty
/// collection is returned.
///
/// * `root` - the root to populate the `Binding`s from
/// returns the `Binding`s found in the root
pub fn from(root: impl AsRef<Path>) -> Vec<impl Binding> {
    let p = root.as_ref();

    if !p.exists() || !p.is_dir() {
        return Vec::new();
    }

    return p.read_dir().map_or(Vec::new(), |b| {
        return b.filter_map(|c| {
            return c.map_or(None, |c| {
                if !c.path().is_dir() {
                    return None;
                }

                return Some(HashMapBinding::new(c.file_name().to_str().unwrap(), HashMap::new()));
            });
        }).collect();
    });
}

/// Creates a new collection of `Binding`s using the `$SERVICE_BINDING_ROOT` environment variable to determine the file
//  system root.  If the `$SERVICE_BINDING_ROOT` environment variable is not set, an empty collection is returned.  If
//  the directory does not exist, an empty collection is returned.
//
// return the `Binding`s found in `$SERVICE_BINDING_ROOT`
pub fn from_service_binding_root() -> Vec<impl Binding> {
    return match env::var_os(SERVICE_BINDING_ROOT) {
        Some(v) => from(v),
        None => Vec::new(),
    };
}

/// Returns a `Binding` with a given name.  Comparison is case insensitive.
///
/// * `bindings` - the `Binding`s to find in
/// * `name` - the name of the `Binding` to find
/// returns the `Binding` with a given name if it exists.
pub fn find(bindings: Vec<impl Binding>, name: &str) -> Option<impl Binding> {
    return bindings.into_iter()
        .find(|b| b.get_name().eq_ignore_ascii_case(name));
}

/// Returns zero or more `Binding`s with a given type and provider.  If type or provider are `None`, the result is not
/// filtered on that argument.  Comparisons are case-insensitive.
///
/// * `bindings` - the `Binding`s to filter
/// * `binding_type` - the type of the `Binding` to find
/// * `provider` - the provider of the `Binding` to find.
///
/// returns the collection of `Binding`s with a given type and provider
pub fn filter_with_provider(bindings: Vec<impl Binding>, binding_type: Option<&str>, provider: Option<&str>) -> Vec<impl Binding> {
    return bindings.into_iter()
        .filter(|b| {
            if let Some(t) = &binding_type {
                if !b.get_type().unwrap().eq_ignore_ascii_case(t) {
                    return false;
                }
            }

            if let Some(p) = &provider {
                match b.get_provider() {
                    None => return false,
                    Some(q) => if !q.eq_ignore_ascii_case(p) {
                        return false;
                    },
                }
            }

            return true;
        })
        .collect();
}

/// Returns zero or more `Binding`s with a given type.  Equivalent to `filter_with_provider` with a `None` provider.
///
/// * `bindings` - the `Binding`s to filter
/// * `binding_type` - the type of the `Binding` to find
/// returns zero or more `Bindings` with a given type
pub fn filter(bindings: Vec<impl Binding>, binding_type: &str) -> Vec<impl Binding> {
    return filter_with_provider(bindings, Some(binding_type), None);
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use lazy_static::lazy_static;

    use crate::binding::{Binding, CacheBinding, HashMapBinding};
    use crate::bindings;

    lazy_static! {
        static ref MUTEX: Mutex<()> = Mutex::default();
    }

    #[test]
    fn cached() {
        let b = bindings::cached(vec![
            HashMapBinding::new("test-name-1", HashMap::new()),
            HashMapBinding::new("test-name-2", HashMap::new()),
        ]);

        for c in b {
            let d: Box<dyn Any> = Box::new(c);
            let _: &CacheBinding = match d.downcast_ref::<CacheBinding>() {
                Some(b) => b,
                None => panic!()
            };
        }
    }

    #[test]
    fn from_invalid() {
        assert!(bindings::from("missing").is_empty());
    }

    #[test]
    fn from_file() {
        assert!(bindings::from("testdata/additional-file").is_empty());
    }

    #[test]
    fn from_valid() {
        assert_eq!(3, bindings::from("testdata").len());
    }

    #[test]
    fn from_service_binding_root_unset() {
        let g = MUTEX.lock().unwrap();
        assert!(bindings::from_service_binding_root().is_empty());
        drop(g)
    }

    #[test]
    fn from_service_binding_root_set() {
        let g = MUTEX.lock().unwrap();
        let old = env::var_os("SERVICE_BINDING_ROOT");
        env::set_var("SERVICE_BINDING_ROOT", "testdata");

        assert_eq!(3, bindings::from_service_binding_root().len());

        match old {
            None => env::remove_var("SERVICE_BINDING_ROOT"),
            Some(v) => env::set_var("SERVICE_BINDING_ROOT", v),
        }
        drop(g)
    }

    #[test]
    fn find_missing() {
        let b = vec![
            HashMapBinding::new("test-name-1", HashMap::new()),
        ];

        assert!(bindings::find(b, "test-name-2").is_none())
    }

    #[test]
    fn find_valid() {
        let b = vec![
            HashMapBinding::new("test-name-1", HashMap::new()),
            HashMapBinding::new("test-name-2", HashMap::new()),
        ];

        assert_eq!(Some(String::from("test-name-1")), bindings::find(b, "test-name-1").map(|q| q.get_name()))
    }

    #[test]
    fn filter_none() {
        let b = vec![
            HashMapBinding::new("test-name-1", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-1",
            }),
            HashMapBinding::new("test-name-2", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-3", map! {
                "type" => "test-type-2",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-4", map! {
                "type" => "test-type-2",
            }),
        ];

        let q = bindings::filter_with_provider(b, None, None);
        assert_eq!(4, q.len());
    }

    #[test]
    fn filter_type() {
        let b = vec![
            HashMapBinding::new("test-name-1", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-1",
            }),
            HashMapBinding::new("test-name-2", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-3", map! {
                "type" => "test-type-2",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-4", map! {
                "type" => "test-type-2",
            }),
        ];

        assert_eq!(2, bindings::filter_with_provider(b, Some("test-type-1"), None).len());
    }

    #[test]
    fn filter_provider() {
        let b = vec![
            HashMapBinding::new("test-name-1", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-1",
            }),
            HashMapBinding::new("test-name-2", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-3", map! {
                "type" => "test-type-2",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-4", map! {
                "type" => "test-type-2",
            }),
        ];

        assert_eq!(2, bindings::filter_with_provider(b, None, Some("test-provider-2")).len());
    }

    #[test]
    fn filter_type_and_provider() {
        let b = vec![
            HashMapBinding::new("test-name-1", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-1",
            }),
            HashMapBinding::new("test-name-2", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-3", map! {
                "type" => "test-type-2",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-4", map! {
                "type" => "test-type-2",
            }),
        ];

        assert_eq!(1, bindings::filter_with_provider(b, Some("test-type-1"), Some("test-provider-1")).len());
    }

    #[test]
    fn filter_overload() {
        let b = vec![
            HashMapBinding::new("test-name-1", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-1",
            }),
            HashMapBinding::new("test-name-2", map! {
                "type" => "test-type-1",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-3", map! {
                "type" => "test-type-2",
                "provider" => "test-provider-2",
            }),
            HashMapBinding::new("test-name-4", map! {
                "type" => "test-type-2",
            }),
        ];

        assert_eq!(2, bindings::filter(b, "test-type-1").len());
    }
}
