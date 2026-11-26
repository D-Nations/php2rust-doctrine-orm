//! The PHP string functions the inflector calls, written from PHP's manual
//! and checked against PHP 8.4.27 (`tests/data/inflector.json`,
//! oracle/recorders/inflector.php). php2rust-value has none of them yet.
//!
//! Upstream: none

use php_regex::Pcre;

/// `strtolower()`: ASCII only (PHP 8.2+).
pub(crate) fn strtolower(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

/// `ucwords($s, $delimiters)`: the first byte and every byte after a
/// delimiter made ASCII upper case.
pub(crate) fn ucwords(s: &[u8], delimiters: &[u8]) -> Vec<u8> {
    let mut out = s.to_vec();
    let mut after_delimiter = true;
    for byte in &mut out {
        if after_delimiter {
            byte.make_ascii_uppercase();
        }
        after_delimiter = delimiters.contains(byte);
    }
    out
}

/// `lcfirst()`.
pub(crate) fn lcfirst(s: &[u8]) -> Vec<u8> {
    let mut out = s.to_vec();
    if let Some(first) = out.first_mut() {
        first.make_ascii_lowercase();
    }
    out
}

/// `str_replace($search, $replace, $subject)` with arrays: each search
/// string replaced in turn over the whole result of the one before.
pub(crate) fn str_replace(search: &[&[u8]], replace: &[&[u8]], subject: &[u8]) -> Vec<u8> {
    let mut out = subject.to_vec();
    for (i, needle) in search.iter().enumerate() {
        let with = replace.get(i).copied().unwrap_or_default();
        out = replace_all(&out, needle, with);
    }
    out
}

fn replace_all(subject: &[u8], needle: &[u8], with: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return subject.to_vec();
    }
    let mut out = Vec::with_capacity(subject.len());
    let mut i = 0;
    while i < subject.len() {
        if subject[i..].starts_with(needle) {
            out.extend_from_slice(with);
            i += needle.len();
        } else {
            out.push(subject[i]);
            i += 1;
        }
    }
    out
}

/// `strtr($s, $pairs)`: at each place the longest key that matches is
/// replaced, and the replacement is not searched again.
pub(crate) fn strtr_pairs(s: &[u8], pairs: &[(&str, &str)]) -> Vec<u8> {
    let longest = pairs.iter().map(|(from, _)| from.len()).max().unwrap_or(0);
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    'outer: while i < s.len() {
        for len in (1..=longest.min(s.len() - i)).rev() {
            let candidate = &s[i..i + len];
            if let Some((_, to)) = pairs.iter().find(|(from, _)| from.as_bytes() == candidate) {
                out.extend_from_slice(to.as_bytes());
                i += len;
                continue 'outer;
            }
        }
        out.push(s[i]);
        i += 1;
    }
    out
}

/// `strtr($s, $from, $to)`: bytes of `from` mapped to the bytes of `to` at
/// the same place (the shorter length counts).
pub(crate) fn strtr_bytes(s: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    s.iter()
        .map(|b| {
            from.iter()
                .zip(to)
                .rev()
                .find(|(f, _)| *f == b)
                .map_or(*b, |(_, t)| *t)
        })
        .collect()
}

/// `trim($s, $characters)` for a list of single characters.
pub(crate) fn trim_chars<'a>(s: &'a [u8], characters: &[u8]) -> &'a [u8] {
    let start = s
        .iter()
        .position(|b| !characters.contains(b))
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|b| !characters.contains(b))
        .map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

/// `mb_strtolower($s)` with the UTF-8 encoding: Unicode's full lower-case
/// mapping (with the final sigma rule); each invalid byte sequence becomes
/// `?`.
pub(crate) fn mb_strtolower(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for chunk in s.utf8_chunks() {
        out.extend_from_slice(chunk.valid().to_lowercase().as_bytes());
        if !chunk.invalid().is_empty() {
            out.push(b'?');
        }
    }
    out
}

/// `preg_replace($regex, $replacement, $subject)`: `None` for PHP's `null`
/// (an invalid pattern or a failed match).
pub(crate) fn preg_replace(
    regex: Option<&Pcre>,
    replacement: &[u8],
    subject: &[u8],
) -> Option<Vec<u8>> {
    regex?.replace(subject, replacement).ok()
}
