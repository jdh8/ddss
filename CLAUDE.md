# ddss

This crate provides a Rusty high-level API for [bsalita/ddss](https://github.com/bsalita/ddss),
a performance-oriented fork of [DDS](https://github.com/dds-bridge/dds) for
contract bridge. It wraps the raw [`ddss-sys`](https://crates.io/crates/ddss-sys)
FFI behind a `Solver` guard that serializes access to ddss's non-reentrant
global thread pool.

After updating the codebase, please

- Format the code with `cargo fmt`.
- Run the tests with `cargo test --all-features`.
- Update [CHANGELOG.md](CHANGELOG.md) with a summary of the changes and their impact on users.
- Propose a clear and descriptive commit message.
