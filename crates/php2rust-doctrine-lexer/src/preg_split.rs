//! `preg_split($regex, $input, -1, PREG_SPLIT_NO_EMPTY |
//! PREG_SPLIT_DELIM_CAPTURE | PREG_SPLIT_OFFSET_CAPTURE)`, the one call
//! `AbstractLexer::scan()` makes, on top of `php_regex` (which has no
//! `preg_split()` yet). Written from PHP's manual and checked against PHP
//! 8.4.27 (`tests/data/lexer.json`, oracle/recorders/lexer.php).
//!
//! The pieces between matches and the captured groups of each match are
//! returned in subject order, empty ones left out, with their byte offsets.
//! After an empty match PCRE tries again at the same place for a non-empty
//! match (`PCRE2_NOTEMPTY_ATSTART | PCRE2_ANCHORED`) and only then moves one
//! character on. `php_regex` has no such option, so a second pattern does
//! it: `(?=(?<r>[\s\S]*\z))(?:BODY)(?!\k<r>\z)` captures the rest of the
//! subject at the start and refuses to end where the same rest is left,
//! that is where it started; a match that does not start there counts as
//! none. Its first group is that capture, so a body with backreferences
//! (which would now point one group off) gets no second pattern, and an
//! empty match with such a body is refused rather than approximated.
//!
//! Upstream: none

use std::sync::OnceLock;

use php_regex::{Groups, Pcre, PcreError};

/// A compiled split pattern.
#[derive(Debug, Clone)]
pub(crate) struct Splitter {
    regex: Pcre,
    body: Vec<u8>,
    modifiers: Vec<u8>,
    /// The pattern for the retry after an empty match, compiled when first
    /// needed.
    not_empty: OnceLock<Result<Pcre, PcreError>>,
    utf8: bool,
}

/// Why the split has no result.
pub(crate) enum SplitError {
    /// `preg_split()` returned `false`: the subject is not UTF-8 under `/u`,
    /// or the backtracking limit was hit.
    Failed,
    /// An empty match whose retry `php_regex` cannot express.
    Unsupported(PcreError),
}

/// The name of the retry pattern's first group.
const REST: &str = "php2rust_rest";

impl Splitter {
    /// Compiles `/BODY/MODIFIERS`.
    pub(crate) fn new(body: &[u8], modifiers: &[u8]) -> Result<Self, PcreError> {
        let mut pattern = b"/".to_vec();
        pattern.extend_from_slice(body);
        pattern.push(b'/');
        pattern.extend_from_slice(modifiers);
        let regex = Pcre::new(&pattern)?;
        Ok(Self {
            regex,
            body: body.to_vec(),
            modifiers: modifiers.to_vec(),
            not_empty: OnceLock::new(),
            utf8: modifiers.contains(&b'u'),
        })
    }

    /// `(?=(?<r>[\s\S]*\z))(?:BODY)(?!\k<r>\z)`.
    fn compile_not_empty(&self) -> Result<Pcre, PcreError> {
        let (body, modifiers) = (&self.body, &self.modifiers);
        if has_backreference(body) {
            return Err(PcreError::Unsupported(
                "a backreference in a lexer pattern that matches the empty string".into(),
            ));
        }
        let mut pattern = format!("/(?=(?<{REST}>[\\s\\S]*\\z))(?:").into_bytes();
        pattern.extend_from_slice(body);
        if modifiers.contains(&b'x') {
            // End a trailing `#` comment before the group closes.
            pattern.push(b'\n');
        }
        pattern.extend_from_slice(format!(")(?!\\k<{REST}>\\z)/").as_bytes());
        pattern.extend_from_slice(modifiers);
        Pcre::new(&pattern)
    }

    /// The byte ranges of the pieces.
    pub(crate) fn split(&self, subject: &[u8]) -> Result<Vec<(usize, usize)>, SplitError> {
        let mut pieces = Vec::new();
        let mut last = 0;
        let mut offset = 0;
        let mut after_empty = false;
        while offset <= subject.len() {
            let groups = if after_empty {
                match self.not_empty_at(subject, offset)? {
                    Some(groups) => groups,
                    None if offset < subject.len() => {
                        offset += self.char_len(subject, offset);
                        after_empty = false;
                        continue;
                    }
                    None => break,
                }
            } else {
                match self.regex.captures_from(subject, offset) {
                    Ok(Some(groups)) => groups,
                    Ok(None) => break,
                    Err(_) => return Err(SplitError::Failed),
                }
            };
            let Some(&Some((start, end))) = groups.first() else {
                break;
            };
            if start > last {
                pieces.push((last, start));
            }
            for &(group_start, group_end) in groups.iter().skip(1).flatten() {
                if group_end > group_start {
                    pieces.push((group_start, group_end));
                }
            }
            last = end;
            offset = end;
            after_empty = end == start;
        }
        if last < subject.len() {
            pieces.push((last, subject.len()));
        }
        Ok(pieces)
    }

    /// The first non-empty match that starts at `offset`, its groups
    /// numbered as in the split pattern.
    fn not_empty_at(&self, subject: &[u8], offset: usize) -> Result<Option<Groups>, SplitError> {
        let regex = self
            .not_empty
            .get_or_init(|| self.compile_not_empty())
            .as_ref()
            .map_err(|error| SplitError::Unsupported(error.clone()))?;
        let Some(mut groups) = regex
            .captures_from(subject, offset)
            .map_err(|_| SplitError::Failed)?
        else {
            return Ok(None);
        };
        // A match further on is not the anchored one PCRE looks for.
        if !matches!(groups.first(), Some(Some((start, _))) if *start == offset) {
            return Ok(None);
        }
        // Drop the rest-of-subject group.
        groups.remove(1);
        Ok(Some(groups))
    }

    /// The length of the character at `at`: a UTF-8 sequence under `/u` (the
    /// subject was checked to be UTF-8 by the match), else one byte.
    fn char_len(&self, subject: &[u8], at: usize) -> usize {
        if !self.utf8 {
            return 1;
        }
        match subject.get(at).copied().unwrap_or(0) {
            0xF0..=0xFF => 4,
            0xE0..=0xEF => 3,
            0xC0..=0xDF => 2,
            _ => 1,
        }
    }
}

/// Whether the pattern body may refer to a group by number or name
/// (`\1`, `\g`, `\k`, `(?P=`); escapes inside classes are counted too.
fn has_backreference(body: &[u8]) -> bool {
    let mut i = 0;
    while i < body.len() {
        match body[i] {
            b'\\' => {
                if matches!(body.get(i + 1), Some(b'1'..=b'9' | b'g' | b'k')) {
                    return true;
                }
                i += 2;
            }
            b'(' if body[i..].starts_with(b"(?P=") => return true,
            _ => i += 1,
        }
    }
    false
}
