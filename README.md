# brace-expansion

[![Crates.io](https://img.shields.io/crates/v/brace-expansion.svg)](https://crates.io/crates/brace-expansion)
[![Documentation](https://docs.rs/brace-expansion/badge.svg)](https://docs.rs/brace-expansion)
[![CI](https://github.com/trananhtung/brace-expansion/actions/workflows/ci.yml/badge.svg)](https://github.com/trananhtung/brace-expansion/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/brace-expansion.svg)](#license)

**Bash-style brace expansion** — `a{b,c}d` → `["abd", "acd"]`, `{1..3}` →
`["1", "2", "3"]`. A faithful Rust port of the
[`brace-expansion`](https://www.npmjs.com/package/brace-expansion) npm package (the
expander behind `minimatch`/`glob`), which implements `bash`'s rules. Zero
dependencies and `#![no_std]`.

```rust
use brace_expansion::expand;

assert_eq!(expand("a{b,c}d"), ["abd", "acd"]);
assert_eq!(expand("{1..3}"), ["1", "2", "3"]);
assert_eq!(expand("{a..c}"), ["a", "b", "c"]);
assert_eq!(expand("{01..03}"), ["01", "02", "03"]);   // zero-padded
assert_eq!(expand("{1..10..2}"), ["1", "3", "5", "7", "9"]); // step
assert_eq!(expand("a{b,c{d,e}}f"), ["abf", "acdf", "acef"]); // nested
```

## Why brace-expansion?

Brace expansion is the first thing a shell does to a glob, and the JS module that
implements it is one of the most-depended-on packages in existence. Its rules are
deceptively subtle — numeric and alphabetic sequences, optional steps and
zero-padding, nested sets, backslash escaping, and a handful of bash compatibility
quirks. This is that algorithm, ported faithfully: output is identical to the npm
package (verified over thousands of patterns), which in turn matches `bash` for
ordinary patterns.

```toml
[dependencies]
brace-expansion = "0.1"
```

## API

| Item | Purpose |
| --- | --- |
| `expand(pattern)` | Expand a pattern into a `Vec<String>` |
| `expand_with_max(pattern, max)` | Same, with a custom result cap |
| `EXPANSION_MAX` | The default cap (`100_000`) |

## Behavior

- Comma sets (`{a,b,c}`) expand to each member; sets combine as a cross product.
- `{x..y}` is a sequence: numeric (`{1..5}`, `{-3..3}`) or alphabetic (`{a..e}`),
  with an optional step (`{0..10..2}`) and zero-padding (`{01..05}`).
- Sets and sequences nest arbitrarily.
- `\{`, `\}`, `\,`, `\.`, and `\\` are escaped and emitted literally.
- Patterns with no valid expansion (`foo`, `{a}`, `a{}b`) are returned unchanged,
  including bash's leading-`{}` quirk.

Output is identical to the npm package for ordinary patterns (verified over many
thousands of cases). A few deliberate edge differences: sequence endpoints use exact
`i64` arithmetic rather than JavaScript's lossy `f64` (so they stay exact past 2^53),
nesting beyond a few hundred levels degrades gracefully instead of recursing without
bound, and the literal NUL-delimited markers used internally are not expected in
real patterns.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.
