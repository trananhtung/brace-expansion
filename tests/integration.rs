//! Behavioral spec for `brace-expansion`, cross-checked against the npm package
//! and bash.

use brace_expansion::expand;

#[test]
fn comma_sets() {
    assert_eq!(expand("a{b,c}d"), ["abd", "acd"]);
    assert_eq!(expand("{a,b}"), ["a", "b"]);
    assert_eq!(expand("{a,b,}"), ["a", "b"]); // trailing empty dropped at top level
    assert_eq!(expand("{1,2}{3,4}"), ["13", "14", "23", "24"]);
    assert_eq!(
        expand("ab{c,d}ef{g,h}"),
        ["abcefg", "abcefh", "abdefg", "abdefh"]
    );
}

#[test]
fn nested() {
    assert_eq!(expand("{a,b{c,d}}"), ["a", "bc", "bd"]);
    assert_eq!(expand("a{b,c{d,e}}f"), ["abf", "acdf", "acef"]);
    assert_eq!(expand("{a,{b,c}}"), ["a", "b", "c"]);
    assert_eq!(expand("{{a,b},c}"), ["a", "b", "c"]);
    assert_eq!(expand("x{{a,b}}y"), ["x{a}y", "x{b}y"]);
}

#[test]
fn numeric_sequences() {
    assert_eq!(expand("{1..3}"), ["1", "2", "3"]);
    assert_eq!(expand("{3..1}"), ["3", "2", "1"]);
    assert_eq!(expand("{-3..3}"), ["-3", "-2", "-1", "0", "1", "2", "3"]);
    assert_eq!(expand("{2..-2}"), ["2", "1", "0", "-1", "-2"]);
    assert_eq!(expand("{1..10..2}"), ["1", "3", "5", "7", "9"]);
    assert_eq!(expand("{10..1..-2}"), ["10", "8", "6", "4", "2"]);
    assert_eq!(expand("{1..3..0}"), ["1", "2", "3"]); // zero step -> 1
}

#[test]
fn padded_sequences() {
    assert_eq!(expand("{01..03}"), ["01", "02", "03"]);
    assert_eq!(expand("{00..05..2}"), ["00", "02", "04"]);
    assert_eq!(expand("{1..3}{a..b}"), ["1a", "1b", "2a", "2b", "3a", "3b"]);
}

#[test]
fn alpha_sequences() {
    assert_eq!(expand("{a..c}"), ["a", "b", "c"]);
    assert_eq!(expand("{c..a}"), ["c", "b", "a"]);
    assert_eq!(expand("{a..e..2}"), ["a", "c", "e"]);
}

#[test]
fn not_expanded() {
    assert_eq!(expand("foo"), ["foo"]);
    assert_eq!(expand("{}"), ["{}"]);
    assert_eq!(expand("{a}"), ["{a}"]);
    assert_eq!(expand("a{}b"), ["a{}b"]);
    assert_eq!(expand("a{b}c"), ["a{b}c"]);
    assert_eq!(expand(""), Vec::<String>::new());
}

#[test]
fn bash_quirks() {
    // a leading `{}` is preserved (bash 4.3 behavior)
    assert_eq!(expand("{},a}b"), ["{},a}b"]);
    assert_eq!(expand("a{},b}c"), ["a}c", "abc"]);
    // unbalanced trailing brace
    assert_eq!(expand("{a,b}{"), ["a{", "b{"]);
}

#[test]
fn escaping() {
    assert_eq!(expand("a\\{b,c\\}"), ["a{b,c}"]);
    assert_eq!(expand("\\{a,b\\}"), ["{a,b}"]);
    assert_eq!(expand("{a,\\,,b}"), ["a", ",", "b"]);
}

// Regression: extreme sequences must not panic or overflow.
#[test]
fn extreme_sequences_do_not_panic() {
    use brace_expansion::expand_with_max;
    // a sequence running to the i64 boundary
    assert_eq!(
        expand("{9223372036854775807..9223372036854775807}").len(),
        1
    );
    assert_eq!(
        expand("{9223372036854775800..9223372036854775807}").len(),
        8
    );
    // out-of-i64 endpoint: capped like node, starting from the low end
    let r = expand("{1..99999999999999999999}");
    assert_eq!(r.len(), brace_expansion::EXPANSION_MAX);
    assert_eq!(r[0], "1");
    // huge in-range sequence is bounded by max
    assert_eq!(expand_with_max("{1..1000000}", 10).len(), 10);
    // an i64::MIN-magnitude step must not overflow `.abs()`
    assert_eq!(expand("{1..5..-9223372036854775808}").len(), 1);
    assert_eq!(expand("{a..z..-9223372036854775808}").len(), 1);
}

#[test]
fn deep_nesting_does_not_abort() {
    use brace_expansion::expand_with_max;
    // recursion is capped, so very deep nesting returns instead of overflowing the
    // stack (without the cap, ~2000 frames overflow a 2 MiB test-thread stack). A
    // sweep up to 200_000 levels was verified manually to also not abort.
    let deep = format!("{}b{}", "{a,".repeat(2000), "}".repeat(2000));
    assert!(!expand(&deep).is_empty());
    // sequential braces recurse via `post`; bound the cross-product with a small max
    let seq = "{a,b}".repeat(2000);
    assert!(!expand_with_max(&seq, 50).is_empty());
}

#[test]
fn dollar_is_literal() {
    // `${...}` looks like a shell variable, so it is not expanded
    assert_eq!(expand("${1,2}"), ["${1,2}"]);
}
