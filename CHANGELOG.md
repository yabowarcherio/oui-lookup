# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0]

### Added

- `entries()` iterator and `Entry` type for walking the whole registry.
- `search()` for case-insensitive vendor-name lookup.
- `lookup_octets()` for pre-parsed `[u8; 6]` addresses.
- `normalize_mac()` to canonicalize any MAC spelling.
- CLI `--search` flag with `--limit`.
- `lookup_entry`, `count_matching` helpers and `Entry: Display`.

## [0.2.0]

### Added

- `parse_mac48` / `format_mac48` for full 48-bit addresses.
- `to_eui64` for IPv6 Modified EUI-64 interface identifiers.
- `is_multicast`, `is_locally_administered`, `is_broadcast` bit helpers.
- `MacKind` enum and `classify()` for coarse address classification.
- `lookup_many` batch helper and `is_registered` predicate.
- CLI `--count` and `--class` flags; black-box CLI tests.

## [0.1.0]

Initial release.

### Added

- `lookup`, `try_lookup`, and `lookup_vendor` library functions for resolving a
  MAC address or OUI prefix to its registered vendor.
- `parse_oui` / `format_oui` helpers and a `ParseMacError` type. Accepts colon,
  hyphen, Cisco-dotted, and bare MAC spellings, case-insensitively.
- The full IEEE MA-L registry, vendored as `data/oui.tsv.gz` and re-packed into
  a sorted binary blob at build time for offline, allocation-free lookups.
- `oui-lookup` CLI with human-readable and `--json` output, stdin support
  (`-`), a `--quiet` flag, and meaningful exit codes.
- `serde` feature for deriving serde traits on the `Vendor` type.
- Criterion benchmark for the lookup hot path.

[Unreleased]: https://github.com/yabowarcherio/oui-lookup/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/yabowarcherio/oui-lookup/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/yabowarcherio/oui-lookup/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yabowarcherio/oui-lookup/releases/tag/v0.1.0
