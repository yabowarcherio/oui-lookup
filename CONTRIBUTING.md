# Contributing to oui-lookup

Thanks for taking the time to contribute! This is a small, focused crate, so
the bar is mostly "keep it fast, offline, and well-tested."

## Getting started

```sh
git clone https://github.com/yabowarcherio/oui-lookup
cd oui-lookup
cargo test
```

You need a recent stable Rust toolchain (see `rust-version` in
[`Cargo.toml`](Cargo.toml) for the minimum supported version, MSRV).

## Before you open a PR

Please make sure the following all pass locally — CI runs the same checks:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --no-default-features        # library-only must still build
```

## Updating the OUI database

The IEEE registry snapshot lives in `data/oui.tsv.gz` and is **vendored on
purpose** so that builds never need the network. Do not add a network fetch to
`build.rs`.

To refresh it:

```sh
./scripts/update-oui.sh
git add data/oui.tsv.gz
git commit -m "data: refresh IEEE OUI snapshot"
```

A scheduled GitHub Actions workflow already does this and opens a PR, so manual
refreshes are rarely necessary.

## Guidelines

- **Keep lookups allocation-free** on the hot path. The embedded blob is
  designed to be searched in place; please don't introduce a runtime
  `HashMap`/`Vec` rebuild.
- **No `unsafe`.** The crate sets `#![forbid(unsafe_code)]`; keep it that way.
- **Add a test** for any behavior change. Integration tests should assert
  *behavior*, not specific vendor strings (those change when the registry is
  refreshed).
- **Document public items.** `missing_docs` is a warning; keep the API
  documented.

## Reporting bugs

Open an issue with the input you used, what you expected, and what happened.
For a wrong/missing vendor, include the MAC prefix — but note the data comes
straight from IEEE, so the fix is usually a database refresh.

## Code of Conduct

Be kind and constructive. We follow the spirit of the
[Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).
