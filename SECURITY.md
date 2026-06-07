# Security Policy

## Reporting a vulnerability

This crate parses untrusted strings (MAC addresses) and embeds a static data
table; it performs no I/O at runtime. If you nonetheless find a memory-safety
or denial-of-service issue, please report it privately via GitHub's
["Report a vulnerability"](https://github.com/yabowarcherio/oui-lookup/security/advisories/new)
flow rather than opening a public issue.

The crate is `#![forbid(unsafe_code)]`, so any soundness bug is necessarily in a
dependency — please include the dependency and version in your report.

## Supported versions

The latest released `0.x` line receives fixes.
