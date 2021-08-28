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

use postgres::{Client, NoTls};

use service_bindings::binding::Binding;
use service_bindings::bindings;

fn main() {
    let b = bindings::from_service_binding_root();
    let c = bindings::filter(b, "postgresql");

    if c.len() != 1 {
        panic!("Incorrect number of PostgreSQL bindings: {}", c.len())
    }

    let u = c[0].get("url");
    let _conn = match u {
        None => panic!("No URL in binding"),
        Some(u) => Client::connect(&u, NoTls),
    };

    // ...
}
