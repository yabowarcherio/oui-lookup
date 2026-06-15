# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Reserved-range predicates: `is_ipv4_multicast` (01:00:5E:00:00:00/25,
  RFC 1112), `is_ipv6_multicast` (33:33::/24, RFC 2464), `is_vrrp`
  (00:00:5E:00:01:xx, RFC 5798), `is_bridge_protocol` (01:80:C2:00:00:00/4),
  `is_pause` (01:80:C2:00:00:01), and `is_stp_bpdu` (01:80:C2:00:00:00).
- `MacScope` enum and `scope(octets)` for fine-grained classification, plus
  `MacScope::is_multicast`/`is_unicast`/`is_specific`/`as_str` predicates.
- Const prefixes: `IPV4_MULTICAST_PREFIX`, `IPV6_MULTICAST_PREFIX`,
  `VRRP_PREFIX`, `BRIDGE_PROTOCOL_PREFIX`.
- `lookup_oui_octets([u8; 3])` direct three-byte prefix lookup.
- Registry stats: `total_vendors()`, `vendor_block_count(name)`, and
  `top_vendors(n)`.
- IPv6 helpers: `mac_from_link_local(addr)` (inverse of `link_local_ipv6`),
  `solicited_node_mac(addr)` (RFC 4861 §7.1), and `lookup_link_local(addr)`
  for one-shot vendor resolution from a link-local IPv6 address.
- CLI: `--scope` for fine-grained classification, `--stats N` for top vendors,
  `--solicited-node` for RFC 4861 MAC derivation.

## [0.5.0]

### Added

- Address helpers: `is_unicast`, `is_zero`, `eui64_to_mac` (inverse of
  `to_eui64`), and `MacKind::is_unicast`/`is_global`/`is_group`.
- Formatters: `format_mac48_hyphen`, `format_mac48_cisco`, `format_mac48_lower`,
  `format_mac48_bare` (no separators), and `format_oui_lower`.
- Lookup: `lookup_oui` for raw integer OUI prefixes; `vendors()` listing the
  distinct registry vendor names.
- Octet/byte lookups: `lookup_entry_octets`, `lookup_vendor_octets`,
  `lookup_octets_many`, and `Entry::octets()`.
- `prefixes_for(name)` to enumerate every OUI of an exact vendor name.
- `MacKind::as_str()` (non-allocating) and `normalize_mac_lower`.
- `link_local_ipv6()` deriving the `fe80::/64` address from a MAC.
- `oui_to_octets` / `octets_to_oui` — const conversion helpers between a
  24-bit OUI integer and its three-byte form.
- `Vendor::octets()` and `Vendor::oui()` to recover the raw prefix from a
  resolved vendor.
- CLI: `--vendor-only`, `--unique`, `--eui64`, `--vendors`, `--normalize`,
  `--lower`, `--link-local`, and `--format bare`.

## [0.4.0]

### Added

- `lookup_vendor_many` for batched lookups returning owned `Vendor` values.
- CLI `--format text|tsv|csv` to control machine-readable output.
- CLI `--input`/`-i FILE` to read addresses from one or more files (repeatable;
  `-` for stdin, blank lines and `#` comments ignored).

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

[Unreleased]: https://github.com/yabowarcherio/oui-lookup/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/yabowarcherio/oui-lookup/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/yabowarcherio/oui-lookup/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/yabowarcherio/oui-lookup/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/yabowarcherio/oui-lookup/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yabowarcherio/oui-lookup/releases/tag/v0.1.0
