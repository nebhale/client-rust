# client-rust

[![Tests](https://github.com/nebhale/client-rust/workflows/Tests/badge.svg?branch=main)](https://github.com/nebhale/client-rust/actions/workflows/tests.yaml)
[![codecov](https://codecov.io/gh/nebhale/client-rust/branch/main/graph/badge.svg)](https://codecov.io/gh/nebhale/client-rust)

`client-rust` is a library to access [Service Binding Specification for Kubernetes](https://k8s-service-bindings.github.io/spec/) conformant Service Binding [Workload Projections](https://k8s-service-bindings.github.io/spec/#workload-projection).

## Example

```rust
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
    let conn = match u {
        None => panic!("No URL in binding"),
        Some(u) => Client::connect(u, NoTls),
    };

    // ...
}
```

## License

Apache License v2.0: see [LICENSE](./LICENSE) for details.
