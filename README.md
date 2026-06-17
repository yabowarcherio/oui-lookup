# oui-lookup

[![CI](https://github.com/yabowarcherio/oui-lookup/actions/workflows/ci.yml/badge.svg)](https://github.com/yabowarcherio/oui-lookup/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/yabowarcherio/oui-lookup?sort=semver)](https://github.com/yabowarcherio/oui-lookup/releases)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](Cargo.toml)

**Fast, offline MAC-address vendor (OUI) lookup — as a Rust library and a CLI.**

The entire IEEE MA-L registry is compiled straight into the binary, so there
are **no network calls, no cache files, and no runtime dependencies**. A lookup
is an allocation-free binary search over data borrowed directly from the
embedded table.

```console
$ oui-lookup a4:83:e7:9c:1d:42
A4:83:E7             Apple, Inc.

$ oui-lookup --json 28:cf:e9 3c:5a:b4
[
  { "input": "28:cf:e9", "prefix": "28:CF:E9", "vendor": "Apple, Inc." },
  { "input": "3c:5a:b4", "prefix": "3C:5A:B4", "vendor": "Google, Inc." }
]
```

## Why another OUI tool?

- **Truly offline.** The database ships *inside* the binary. No first-run
  download, no `~/.cache` directory, no failure when you're on an air-gapped
  network. Drop the single ~2 MB executable anywhere and it just works.
- **Tiny and fast.** Lookups are a binary search over a packed blob — typically
  tens of nanoseconds. No hash map to build, nothing to allocate.
- **Library *and* CLI.** Use it from the command line, or depend on the crate
  and call `oui_lookup::lookup()` directly. (It's the vendor-resolution core of
  the [NetScan](https://github.com/yabowarcherio) network scanner.)
- **Reproducible builds.** The IEEE data is vendored into the repository, so
  `cargo build` never touches the network and CI is deterministic.

## Install

### As a CLI

Grab a prebuilt binary from the [releases page](https://github.com/yabowarcherio/oui-lookup/releases),
or build from source:

```sh
cargo install --git https://github.com/yabowarcherio/oui-lookup
```

### As a library

Add it from git:

```toml
[dependencies]
oui-lookup = { git = "https://github.com/yabowarcherio/oui-lookup", default-features = false }
```

`default-features = false` drops the CLI's dependencies (clap, serde_json) and
gives you a lean library. Re-enable serde derives on the `Vendor` type with
`features = ["serde"]`.

## Usage (CLI)

```text
oui-lookup [OPTIONS] <MAC>...

Arguments:
  <MAC>...  MAC addresses or OUI prefixes to look up. Use `-` to read from stdin.

Options:
      --json            Emit results as a JSON array
      --format <FORMAT> Output format: text (default), tsv, csv, or bare
  -i, --input <FILE>    Read addresses from a file, one per line (repeatable)
      --quiet           Suppress "(unknown)" lines for unmatched addresses
      --class           Also print the address class (unicast/multicast/...)
      --vendor-only     Print only the vendor name per input (blank if unknown)
      --unique          Drop duplicate inputs, keeping the first occurrence
      --eui64           Print the Modified EUI-64 identifier per full MAC
      --link-local      Print the IPv6 link-local (fe80::/64) per full MAC
      --normalize       Print the canonical form of each full MAC
      --lower           Use the lower-case colon form (with --normalize/--eui64)
      --vendors         Print every distinct vendor name, sorted, then exit
      --search <TEXT>   Print OUIs whose vendor name contains TEXT, then exit
      --limit <N>       Cap the rows printed by --search (0 = no limit)
      --count           Print how many OUIs are embedded, then exit
  -h, --help            Print help
  -V, --version
```

Any common MAC spelling is accepted — only the first three octets matter:

```sh
oui-lookup 00:11:22:33:44:55     # colons
oui-lookup 00-11-22-33-44-55     # hyphens
oui-lookup 0011.2233.4455        # Cisco dotted
oui-lookup 001122334455          # bare
oui-lookup 00:11:22              # just the OUI
```

Read many addresses from stdin, a file, or both:

```sh
arp -a | grep -oE '([0-9a-f]{2}:){5}[0-9a-f]{2}' | oui-lookup -
oui-lookup --input macs.txt              # one address per line; # comments ok
oui-lookup --format csv -i macs.txt > vendors.csv
```

Show the address class alongside the vendor:

```sh
oui-lookup --class a4:83:e7:9c:1d:42
# A4:83:E7             global-unicast   Apple, Inc.
```

**Exit codes:** `0` all matched · `1` parsed but at least one unknown vendor ·
`2` at least one input failed to parse.

## Usage (library)

```rust
use oui_lookup::{lookup, lookup_vendor, try_lookup};

// Simplest form: Option<&'static str>, borrowed from the embedded table.
assert_eq!(lookup("a4:83:e7:00:00:00"), Some("Apple, Inc."));

// Distinguish "unparseable" from "parsed but unknown".
assert!(try_lookup("not-a-mac").is_err());
assert_eq!(try_lookup("ff:ff:ff:00:00:00"), Ok(None));

// Owned prefix + name, ready to serialize.
let v = lookup_vendor("28:cf:e9:11:22:33").unwrap();
assert_eq!(v.prefix, "28:CF:E9");
```

## More library helpers

```rust
use oui_lookup::{lookup_many, lookup_vendor_many, is_registered, parse_mac48, to_eui64, is_multicast};

// Batch lookups, order preserved.
let names = lookup_many(["a4:83:e7:00:00:00", "28:cf:e9:11:22:33"]);
assert_eq!(names.len(), 2);

// Same, but each result is an owned `Vendor` (prefix + name), ready to serialize.
let vendors = lookup_vendor_many(["a4:83:e7:00:00:00", "28:cf:e9:11:22:33"]);
assert_eq!(vendors.len(), 2);

// Cheap membership check.
assert!(is_registered("a4:83:e7:00:00:00") || !is_registered("a4:83:e7:00:00:00"));

// Full-address utilities.
let mac = parse_mac48("01:00:5e:00:00:01").unwrap();
assert!(is_multicast(mac));
let _eui64 = to_eui64(mac);
```

## Registry stats

```rust
use oui_lookup::{top_vendors, total_vendors, vendor_block_count};

let n_vendors = total_vendors();
let apple_blocks = vendor_block_count("Apple, Inc.");
for (name, count) in top_vendors(5) {
    println!("{count}\t{name}");
}
# let _ = (n_vendors, apple_blocks);
```

From the CLI:

```sh
oui-lookup --stats 10        # top-10 vendors by OUI-block count
```

## IPv6 neighbor helpers

```rust
use oui_lookup::{lookup_link_local, mac_from_link_local, solicited_node_mac};

let ll: std::net::Ipv6Addr = "fe80::a483:e7ff:fe11:2233".parse().unwrap();
// Recover the underlying MAC and resolve its vendor in one call.
let _vendor = lookup_link_local(ll);
let _mac    = mac_from_link_local(ll);
// Solicited-node multicast destination per RFC 4861 §7.1.
assert_eq!(solicited_node_mac(ll), [0x33, 0x33, 0xFF, 0x11, 0x22, 0x33]);
```

From the CLI:

```sh
oui-lookup --solicited-node fe80::a483:e7ff:fe11:2233
```

## Address scope

`MacScope` is a finer classification than `MacKind`: it picks out the well-known
reserved blocks (IPv4/IPv6 multicast, VRRP, bridge-protocol) instead of just
unicast/multicast/broadcast.

```rust
use oui_lookup::{parse_mac48, scope, MacScope};

assert_eq!(scope(parse_mac48("01:00:5e:00:00:01").unwrap()), MacScope::Ipv4Multicast);
assert_eq!(scope(parse_mac48("33:33:00:00:00:01").unwrap()), MacScope::Ipv6Multicast);
assert_eq!(scope(parse_mac48("01:80:c2:00:00:00").unwrap()), MacScope::BridgeProtocol);
assert_eq!(scope(parse_mac48("00:00:5e:00:01:0a").unwrap()), MacScope::Vrrp);
```

From the CLI:

```sh
oui-lookup --scope 01:00:5e:00:00:01     # ipv4-multicast
```

## Formats and prefix conversions

```rust
use oui_lookup::{
    format_mac48, format_mac48_bare, format_mac48_cisco, format_mac48_hyphen,
    format_mac48_lower, octets_to_oui, oui_to_octets, parse_mac48,
};

let m = parse_mac48("00:11:22:33:44:55").unwrap();
assert_eq!(format_mac48(m),        "00:11:22:33:44:55");
assert_eq!(format_mac48_lower(m),  "00:11:22:33:44:55");
assert_eq!(format_mac48_hyphen(m), "00-11-22-33-44-55");
assert_eq!(format_mac48_cisco(m),  "0011.2233.4455");
assert_eq!(format_mac48_bare(m),   "001122334455");

// 24-bit OUI <-> three bytes (both are `const fn`).
assert_eq!(octets_to_oui([0xA4, 0x83, 0xE7]), 0xA483E7);
assert_eq!(oui_to_octets(0xA483E7), [0xA4, 0x83, 0xE7]);
```

## Searching and iterating

```rust
use oui_lookup::{search, entries};

// Find every OUI registered to a vendor (case-insensitive substring).
for e in search("raspberry") {
    println!("{} {}", e.prefix_str(), e.name);
}

// Or walk the entire embedded table.
let total = entries().count();
assert!(total > 10_000);
```

From the CLI:

```sh
oui-lookup --search "raspberry pi"
oui-lookup --count          # how many OUIs are embedded
```

## Quick counts

```rust
use oui_lookup::{count_matching, lookup_entry};

let apple_blocks = count_matching("apple");
println!("Apple holds {apple_blocks} OUI blocks");

if let Some(e) = lookup_entry("a4:83:e7:00:00:00") {
    println!("{}", e); // "A4:83:E7  Apple, Inc."
}
```

## How the data is embedded

1. `data/oui.tsv.gz` — a gzip-compressed `PREFIX\tVendor` snapshot of the IEEE
   MA-L registry, committed to the repo.
2. [`build.rs`](build.rs) decompresses it at compile time and re-packs it into a
   sorted binary blob (`magic | count | (prefix, offset)[] | string-pool`).
3. The blob is `include_bytes!`-d into the crate and binary-searched at runtime.

This keeps `cargo build` fully offline. To refresh the snapshot from IEEE, run
[`scripts/update-oui.sh`](scripts/update-oui.sh) and commit the result. A
scheduled GitHub Actions workflow does this automatically and opens a PR.

> **Scope:** only the MA-L (`/24`) registry is embedded today. Finer MA-M
> (`/28`) and MA-S (`/36`) sub-allocations are not yet resolved.

## Contributing

Issues and PRs are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

The embedded OUI data is published by the IEEE Registration Authority and is
redistributed here for convenience; it is not covered by the licenses above.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
