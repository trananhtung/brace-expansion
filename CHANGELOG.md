# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-22

### Added

- Initial release.
- `expand` / `expand_with_max` — bash-style brace expansion.
- `EXPANSION_MAX` constant (the default result cap).
- Faithful to the `brace-expansion` npm package: comma sets, numeric and alphabetic
  sequences with steps and zero-padding, arbitrary nesting, backslash escaping, and
  bash compatibility quirks. Zero dependencies; `#![no_std]`.

[0.1.0]: https://github.com/trananhtung/brace-expansion/releases/tag/v0.1.0
