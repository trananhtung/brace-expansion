//! # brace-expansion — bash-style brace expansion
//!
//! Expand brace patterns the way `bash` does: `a{b,c}d` becomes `["abd", "acd"]`,
//! `{1..3}` becomes `["1", "2", "3"]`, with nested sets, numeric and alphabetic
//! sequences (with an optional step and zero-padding), and backslash escaping. A
//! faithful Rust port of the [`brace-expansion`](https://www.npmjs.com/package/brace-expansion)
//! npm package (the expander behind `minimatch`/`glob`). Zero dependencies and
//! `#![no_std]`.
//!
//! ```
//! use brace_expansion::expand;
//!
//! assert_eq!(expand("a{b,c}d"), ["abd", "acd"]);
//! assert_eq!(expand("{1..3}"), ["1", "2", "3"]);
//! assert_eq!(expand("{a..c}"), ["a", "b", "c"]);
//! assert_eq!(expand("{01..03}"), ["01", "02", "03"]);
//! assert_eq!(expand("a{b,c{d,e}}f"), ["abf", "acdf", "acef"]);
//! ```

#![no_std]
#![doc(html_root_url = "https://docs.rs/brace-expansion/0.1.0")]
// Index arithmetic mirrors balanced-match's `-1`/`indexOf` logic; every cast is on a
// value already bounded by the input length.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

// Compile-test the README's examples as part of `cargo test`.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// The default cap on the number of generated results, matching the npm package.
pub const EXPANSION_MAX: usize = 100_000;

/// A bound on brace-nesting recursion depth, so pathologically nested input degrades
/// gracefully instead of overflowing the stack. Kept well below what a small (2 MiB)
/// thread stack tolerates; real patterns nest only a handful of levels.
const MAX_DEPTH: u32 = 256;

// Sentinels used to hide escaped characters during expansion. They are wrapped in
// NUL bytes, which do not appear in realistic brace patterns (an input that
// literally contains these markers would round-trip incorrectly).
const ESC_SLASH: &str = "\u{0}SLASH\u{0}";
const ESC_OPEN: &str = "\u{0}OPEN\u{0}";
const ESC_CLOSE: &str = "\u{0}CLOSE\u{0}";
const ESC_COMMA: &str = "\u{0}COMMA\u{0}";
const ESC_PERIOD: &str = "\u{0}PERIOD\u{0}";

/// Expand a brace pattern, returning every resulting string in order.
///
/// An empty input yields an empty list. The number of results is capped at
/// [`EXPANSION_MAX`]; use [`expand_with_max`] to choose a different limit.
///
/// ```
/// assert_eq!(brace_expansion::expand("{a,b}{c,d}"), ["ac", "ad", "bc", "bd"]);
/// ```
#[must_use]
pub fn expand(pattern: &str) -> Vec<String> {
    expand_with_max(pattern, EXPANSION_MAX)
}

/// Expand a brace pattern, capping the number of results at `max`.
#[must_use]
pub fn expand_with_max(pattern: &str, max: usize) -> Vec<String> {
    if pattern.is_empty() {
        return Vec::new();
    }
    // Bash 4.3 preserves a leading `{}`; mirror that by escaping it.
    let pattern = match pattern.strip_prefix("{}") {
        Some(rest) => format!("\\{{\\}}{rest}"),
        None => pattern.to_string(),
    };

    expand_inner(&escape_braces(&pattern), max, true, 0)
        .iter()
        .map(|s| unescape_braces(s))
        .collect()
}

fn escape_braces(s: &str) -> String {
    // Order matters: collapse escaped backslashes (`\\`) first, then `\{ \} \, \.`.
    s.replace("\\\\", ESC_SLASH)
        .replace("\\{", ESC_OPEN)
        .replace("\\}", ESC_CLOSE)
        .replace("\\,", ESC_COMMA)
        .replace("\\.", ESC_PERIOD)
}

fn unescape_braces(s: &str) -> String {
    s.replace(ESC_SLASH, "\\")
        .replace(ESC_OPEN, "{")
        .replace(ESC_CLOSE, "}")
        .replace(ESC_COMMA, ",")
        .replace(ESC_PERIOD, ".")
}

fn embrace(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('{');
    out.push_str(s);
    out.push('}');
    out
}

/// `parseInt`-style numeric value: the integer if it parses, otherwise the first
/// character's code point. Integers beyond `i64` saturate (they only occur in
/// pathological sequences, which the result cap bounds anyway).
fn numeric(s: &str) -> i64 {
    match s.parse::<i64>() {
        Ok(n) => n,
        Err(_) if is_int(s) => {
            if s.starts_with('-') {
                i64::MIN
            } else {
                i64::MAX
            }
        }
        Err(_) => s.chars().next().map_or(0, |c| c as i64),
    }
}

/// Whether `el` is zero-padded (`/^-?0\d/`).
fn is_padded(el: &str) -> bool {
    let body = el.strip_prefix('-').unwrap_or(el).as_bytes();
    body.len() >= 2 && body[0] == b'0' && body[1].is_ascii_digit()
}

fn is_int(s: &str) -> bool {
    let body = s.strip_prefix('-').unwrap_or(s);
    !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit())
}

fn is_single_letter(s: &str) -> bool {
    let mut chars = s.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if c.is_ascii_alphabetic())
}

fn is_numeric_sequence(body: &str) -> bool {
    let parts: Vec<&str> = body.split("..").collect();
    matches!(parts.len(), 2 | 3) && parts.iter().all(|p| is_int(p))
}

fn is_alpha_sequence(body: &str) -> bool {
    let parts: Vec<&str> = body.split("..").collect();
    match parts.len() {
        2 => is_single_letter(parts[0]) && is_single_letter(parts[1]),
        3 => is_single_letter(parts[0]) && is_single_letter(parts[1]) && is_int(parts[2]),
        _ => false,
    }
}

/// Whether `post` matches `/,(?!,).*\}/`: a comma not followed by a comma, with a
/// `}` somewhere after it.
fn has_dangling_close(post: &str) -> bool {
    let chars: Vec<char> = post.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == ',' && chars.get(i + 1) != Some(&',') && chars[i + 1..].contains(&'}') {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// balanced-match: find the first balanced `{ … }`
// ---------------------------------------------------------------------------

/// Find the first balanced `{ … }` and return `(pre, body, post)`.
fn balanced(s: &str) -> Option<(String, String, String)> {
    let chars: Vec<char> = s.chars().collect();
    let (start, end) = range(&chars)?;
    Some((
        chars[..start].iter().collect(),
        chars[start + 1..end].iter().collect(),
        chars[end + 1..].iter().collect(),
    ))
}

fn index_of(s: &[char], c: char, from: i64) -> i64 {
    let start = if from < 0 { 0 } else { from as usize };
    if start >= s.len() {
        return -1;
    }
    match s[start..].iter().position(|&x| x == c) {
        Some(p) => (start + p) as i64,
        None => -1,
    }
}

/// Port of balanced-match's `range` for `a = '{'`, `b = '}'`.
#[allow(clippy::many_single_char_names)] // names mirror the reference implementation
fn range(s: &[char]) -> Option<(usize, usize)> {
    let (a, b) = ('{', '}');
    let mut ai = index_of(s, a, 0);
    let mut bi = index_of(s, b, ai + 1);
    let mut i = ai;
    let mut result: Option<(i64, i64)> = None;

    if ai >= 0 && bi > 0 {
        let mut begs: Vec<i64> = Vec::new();
        let mut left = s.len() as i64;
        let mut right: i64 = -1;
        let mut right_set = false;

        while i >= 0 && result.is_none() {
            if i == ai {
                begs.push(i);
                ai = index_of(s, a, i + 1);
            } else if begs.len() == 1 {
                if let Some(r) = begs.pop() {
                    result = Some((r, bi));
                }
            } else {
                if let Some(beg) = begs.pop() {
                    if beg < left {
                        left = beg;
                        right = bi;
                        right_set = true;
                    }
                }
                bi = index_of(s, b, i + 1);
            }
            i = if ai < bi && ai >= 0 { ai } else { bi };
        }

        if !begs.is_empty() && right_set {
            result = Some((left, right));
        }
    }

    result.map(|(l, r)| (l as usize, r as usize))
}

/// Split `str` on top-level commas, treating nested `{ … }` sets as single members.
fn parse_comma_parts(s: &str, depth: u32) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    let split_plain = || s.split(',').map(ToString::to_string).collect();
    if depth > MAX_DEPTH {
        return split_plain();
    }
    let Some((pre, body, post)) = balanced(s) else {
        return split_plain();
    };

    let mut p: Vec<String> = pre.split(',').map(ToString::to_string).collect();
    let last = p.len() - 1;
    p[last].push('{');
    p[last].push_str(&body);
    p[last].push('}');

    let mut post_parts = parse_comma_parts(&post, depth + 1);
    if !post.is_empty() && !post_parts.is_empty() {
        let first = post_parts.remove(0);
        p[last].push_str(&first);
        p.extend(post_parts);
    }
    p
}

fn expand_inner(s: &str, max: usize, is_top: bool, depth: u32) -> Vec<String> {
    if depth > MAX_DEPTH {
        return vec![s.to_string()];
    }
    let Some((pre, body, post_str)) = balanced(s) else {
        return vec![s.to_string()];
    };

    let post = if post_str.is_empty() {
        vec![String::new()]
    } else {
        expand_inner(&post_str, max, false, depth + 1)
    };

    let mut expansions: Vec<String> = Vec::new();

    if pre.ends_with('$') {
        for p in post.iter().take(max) {
            expansions.push(format!("{pre}{{{body}}}{p}"));
        }
        return expansions;
    }

    let is_numeric_seq = is_numeric_sequence(&body);
    let is_alpha_seq = is_alpha_sequence(&body);
    let is_sequence = is_numeric_seq || is_alpha_seq;
    let is_options = body.contains(',');

    if !is_sequence && !is_options {
        if has_dangling_close(&post_str) {
            let rebuilt = format!("{pre}{{{body}{ESC_CLOSE}{post_str}");
            return expand_inner(&rebuilt, max, true, depth + 1);
        }
        return vec![s.to_string()];
    }

    let n: Vec<String> = if is_sequence {
        body.split("..").map(ToString::to_string).collect()
    } else {
        let parsed = parse_comma_parts(&body, depth + 1);
        if parsed.len() == 1 {
            let embraced: Vec<String> = expand_inner(&parsed[0], max, false, depth + 1)
                .iter()
                .map(|x| embrace(x))
                .collect();
            if embraced.len() == 1 {
                return post
                    .iter()
                    .map(|p| format!("{pre}{}{p}", embraced[0]))
                    .collect();
            }
            embraced
        } else {
            parsed
        }
    };

    let big_n: Vec<String> = if is_sequence {
        sequence(&n, is_alpha_seq, max)
    } else {
        let mut v = Vec::new();
        for part in &n {
            v.extend(expand_inner(part, max, false, depth + 1));
        }
        v
    };

    for item in &big_n {
        for p in &post {
            if expansions.len() >= max {
                break;
            }
            let expansion = format!("{pre}{item}{p}");
            if !is_top || is_sequence || !expansion.is_empty() {
                expansions.push(expansion);
            }
        }
    }

    expansions
}

/// Generate the members of a numeric or alphabetic sequence (`n` is the `..`-split
/// body, length 2 or 3).
#[allow(clippy::many_single_char_names)] // names mirror the reference implementation
fn sequence(n: &[String], is_alpha: bool, max: usize) -> Vec<String> {
    let x = numeric(&n[0]);
    let y = numeric(&n[1]);
    let width = n[0].chars().count().max(n[1].chars().count());
    let mut incr: i64 = if n.len() == 3 {
        numeric(&n[2]).saturating_abs().max(1)
    } else {
        1
    };
    let reverse = y < x;
    if reverse {
        incr = -incr;
    }
    let pad = n.iter().any(|e| is_padded(e));

    let mut out = Vec::new();
    let mut i = x;
    loop {
        let in_range = if reverse { i >= y } else { i <= y };
        if !in_range || out.len() >= max {
            break;
        }
        if is_alpha {
            match u32::try_from(i).ok().and_then(char::from_u32) {
                Some('\\') | None => out.push(String::new()),
                Some(c) => out.push(c.to_string()),
            }
        } else {
            out.push(pad_number(i, width, pad));
        }
        // Stop rather than overflow when a sequence runs to the i64 boundary.
        match i.checked_add(incr) {
            Some(next) => i = next,
            None => break,
        }
    }
    out
}

fn pad_number(i: i64, width: usize, pad: bool) -> String {
    let mut c = i.to_string();
    if pad {
        let need = width as i64 - c.chars().count() as i64;
        if need > 0 {
            let zeros = "0".repeat(need as usize);
            c = if i < 0 {
                let digits = c.strip_prefix('-').unwrap_or("").to_string();
                format!("-{zeros}{digits}")
            } else {
                format!("{zeros}{c}")
            };
        }
    }
    c
}
