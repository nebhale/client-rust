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

use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::str;

use crate::secret;

/// The key for the provider of a `Binding`.
pub const PROVIDER: &str = "provider";

/// The key for the type of a `Binding`.
pub const TYPE: &str = "type";

/// An error returned when an invalid `Binding` is encountered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidBindingError {
    message: String,
}

impl InvalidBindingError {
    pub fn new(message: impl Into<String>) -> InvalidBindingError {
        InvalidBindingError { message: message.into() }
    }
}

/// A representation of a binding as defined by the
/// [Kubernetes Service Binding Specification](https://github.com/k8s-service-bindings/spec#workload-projection).
pub trait Binding {
    /// Returns the contents of a `Binding` entry in its raw bytes form.
    ///
    /// * `key` - the key of the entry to retrieve
    ///
    /// returns the contents of a `Binding` entry if it exists, otherwise `None`
    fn get_as_bytes(&self, key: &str) -> Option<Vec<u8>>;

    /// Returns the name of the `Binding`
    ///
    /// returns the name of the `Binding`
    fn get_name(&self) -> String;

    /// Returns the contents of a `Binding` entry as a UTF-8 decoded `str`.  Any whitespace is trimmed.
    ///
    /// * `key` - the key of the entry to retrieve
    ///
    /// returns the contents of a `Binding` entry as a UTF-8 decoded `str` if it exists, otherwise `None`
    fn get(&self, key: &str) -> Option<String> {
        return match self.get_as_bytes(key) {
            None => None,
            Some(b) => Some(str::from_utf8(&b)
                .map(|s| s.trim().to_string())
                .unwrap()),
        };
    }

    /// Returns the value of the `PROVIDER` key.
    ///
    /// returns the value of the `PROVIDER` key if it exists, otherwise `None`
    fn get_provider(&self) -> Option<String> {
        return self.get(PROVIDER);
    }

    /// Returns the value of the `TYPE` key.
    ///
    /// returns the value of the `TYPE` key
    fn get_type(&self) -> Result<String, InvalidBindingError> {
        return match self.get(TYPE) {
            None => Err(InvalidBindingError::new("binding does not contain a type")),
            Some(t) => Ok(t),
        };
    }
}

/// An implementation of `Binding` that caches values once they've been retrieved.
pub struct CacheBinding<'a> {
    delegate: Box<dyn Binding + 'a>,
    cache: RefCell<HashMap<String, Vec<u8>>>,
}

impl<'a> CacheBinding<'a> {
    /// Creates a new instance.
    ///
    /// * `delegate` - the `Binding` used to retrieve the original values
    pub fn new(delegate: impl Binding + 'a) -> CacheBinding<'a> {
        return CacheBinding {
            delegate: Box::new(delegate),
            cache: RefCell::new(HashMap::new()),
        };
    }
}

impl Binding for CacheBinding<'_> {
    fn get_as_bytes(&self, key: &str) -> Option<Vec<u8>> {
        return match self.cache.borrow_mut().entry(key.to_string()) {
            Entry::Occupied(o) => Some(o.get().to_vec()),
            Entry::Vacant(v) => {
                return match self.delegate.get_as_bytes(key) {
                    None => None,
                    Some(w) => Some(v.insert(w).to_vec()),
                };
            }
        };
    }

    fn get_name(&self) -> String {
        return self.delegate.get_name();
    }
}

/// An implementation of `Binding` that reads files from a volume mounted
/// [Kubernetes Secret](https://kubernetes.io/docs/concepts/configuration/secret/#using-secrets).
pub struct ConfigTreeBinding {
    root: PathBuf,
}

impl ConfigTreeBinding {
    /// Creates a new instance.
    ///
    /// * `root` - the root of the volume mounted Kubernetes Secret
    pub fn new<P: Into<PathBuf>>(root: P) -> ConfigTreeBinding {
        return ConfigTreeBinding {
            root: root.into()
        };
    }
}

impl Binding for ConfigTreeBinding {
    fn get_as_bytes(&self, key: &str) -> Option<Vec<u8>> {
        if !secret::is_valid_secret_key(key) {
            return None;
        }

        let p = self.root.join(PathBuf::from(key));

        if !p.exists() || !p.is_file() {
            return None;
        }

        return fs::read(p).ok();
    }

    fn get_name(&self) -> String {
        return self.root.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap();
    }
}

/// An implementation of `Binding` that returns values from a `HashMap`.
pub struct HashMapBinding {
    name: String,
    content: HashMap<String, Vec<u8>>,
}

impl HashMapBinding {
    /// Creates a new instance.
    ///
    /// * `name` - the name of the `Binding`
    /// * `content` - the content of the `Binding`
    pub fn new(name: impl Into<String>, content: HashMap<String, Vec<u8>>) -> HashMapBinding {
        return HashMapBinding {
            name: name.into(),
            content,
        };
    }
}

impl Binding for HashMapBinding {
    fn get_as_bytes(&self, key: &str) -> Option<Vec<u8>> {
        if !secret::is_valid_secret_key(key) {
            return None;
        }

        if !self.content.contains_key(key) {
            return None;
        }

        return self.content.get(key)
            .map(|v| v.to_vec());
    }

    fn get_name(&self) -> String {
        return self.name.to_string();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::binding::{Binding, CacheBinding, ConfigTreeBinding, HashMapBinding, InvalidBindingError};

    #[test]
    fn get_missing() {
        let b = HashMapBinding::new("test-name", map! {});
        assert_eq!(None, b.get("test-missing-key"))
    }

    #[test]
    fn get_valid() {
        let b = HashMapBinding::new("test-name", map! {
            "test-secret-key" => "test-secret-value\n",
        });

        assert_eq!(Some("test-secret-value".to_string()), b.get("test-secret-key"))
    }

    #[test]
    fn get_provider_missing() {
        let b = HashMapBinding::new("test-name", HashMap::new());
        assert_eq!(None, b.get_provider())
    }

    #[test]
    fn get_provider_valid() {
        let b = HashMapBinding::new("test-name", map! {
            "provider" => "test-provider-1",
        });

        assert_eq!(Some("test-provider-1".to_string()), b.get_provider())
    }

    #[test]
    fn get_type_invalid() {
        let b = HashMapBinding::new("test-name", HashMap::new());
        assert_eq!(Err(InvalidBindingError::new("binding does not contain a type")), b.get_type())
    }

    #[test]
    fn get_type_valid() {
        let b = HashMapBinding::new("test-name", map! {
            "type" => "test-type-1",
        });

        assert_eq!(Ok("test-type-1".to_string()), b.get_type())
    }

    #[test]
    fn cache_binding_missing() {
        let s = StubBinding::new();
        let c = Rc::clone(&s.get_as_bytes_count);

        let b = CacheBinding::new(s);

        assert_eq!(None, b.get_as_bytes("test-unknown-key"));
        assert_eq!(None, b.get_as_bytes("test-unknown-key"));
        assert_eq!(2, c.take());
    }

    #[test]
    fn cache_binding_valid() {
        let s = StubBinding::new();
        let c = Rc::clone(&s.get_as_bytes_count);

        let b = CacheBinding::new(s);

        assert_eq!(Some(Vec::new()), b.get_as_bytes("test-secret-key"));
        assert_eq!(Some(Vec::new()), b.get_as_bytes("test-secret-key"));
        assert_eq!(1, c.take());
    }

    #[test]
    fn cache_binding_get_name() {
        let s = StubBinding::new();
        let c = Rc::clone(&s.get_name_count);

        let b = CacheBinding::new(s);

        assert_eq!(String::from("test-name"), b.get_name());
        assert_eq!(String::from("test-name"), b.get_name());
        assert_eq!(2, c.take());
    }

    #[test]
    fn config_tree_binding_missing() {
        let b = ConfigTreeBinding::new("testdata/test-k8s");
        assert_eq!(None, b.get_as_bytes("test-missing-key"))
    }

    #[test]
    fn config_tree_binding_directory() {
        let b = ConfigTreeBinding::new("testdata/test-k8s");
        assert_eq!(None, b.get_as_bytes(".hidden-data"))
    }

    #[test]
    fn config_tree_binding_invalid() {
        let b = ConfigTreeBinding::new("testdata/test-k8s");
        assert_eq!(None, b.get_as_bytes("test^invalid^key"))
    }

    #[test]
    fn config_tree_binding_valid() {
        let b = ConfigTreeBinding::new("testdata/test-k8s");
        assert_eq!(Some("test-secret-value\n".as_bytes().to_vec()), b.get_as_bytes("test-secret-key"))
    }

    #[test]
    fn config_tree_binding_get_name() {
        let b = ConfigTreeBinding::new("testdata/test-k8s");
        assert_eq!(String::from("test-k8s"), b.get_name())
    }

    #[test]
    fn hash_map_binding_missing() {
        let b = HashMapBinding::new("test-name", HashMap::new());
        assert_eq!(None, b.get_as_bytes("test-missing-key"))
    }

    #[test]
    fn hash_map_binding_invalid() {
        let b = HashMapBinding::new("test-name", HashMap::new());
        assert_eq!(None, b.get_as_bytes("test^invalid^key"))
    }

    #[test]
    fn hash_map_binding_valid() {
        let b = HashMapBinding::new("test-name", map! {
            "test-secret-key" => "test-secret-value\n",
        });

        assert_eq!(Some("test-secret-value\n".as_bytes().to_vec()), b.get_as_bytes("test-secret-key"))
    }

    #[test]
    fn hash_map_binding_get_name() {
        let b = HashMapBinding::new("test-name", HashMap::new());
        assert_eq!("test-name", b.get_name())
    }

    struct StubBinding {
        get_as_bytes_count: Rc<RefCell<i32>>,
        get_name_count: Rc<RefCell<i32>>,
    }

    impl StubBinding {
        fn new() -> StubBinding {
            return StubBinding {
                get_as_bytes_count: Rc::new(RefCell::new(0)),
                get_name_count: Rc::new(RefCell::new(0)),
            };
        }
    }

    impl Binding for StubBinding {
        fn get_as_bytes(&self, key: &str) -> Option<Vec<u8>> {
            (*self.get_as_bytes_count).replace_with(|f| *f + 1);

            if "test-secret-key".eq(key) {
                return Some(Vec::new());
            }

            return None;
        }

        fn get_name(&self) -> String {
            (*self.get_name_count).replace_with(|f| *f + 1);
            return String::from("test-name");
        }
    }
}
