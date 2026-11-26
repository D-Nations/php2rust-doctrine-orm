//! Upstream: doctrine/inflector src/Inflector.php

use std::sync::{Arc, OnceLock};

use php_regex::Pcre;
use php_value::PhpString;

use crate::error::InflectorError;
use crate::php::{
    lcfirst, mb_strtolower, preg_replace, str_replace, strtr_bytes, strtr_pairs, trim_chars,
    ucwords,
};
use crate::word_inflector::WordInflector;

const ACCENTED_CHARACTERS: &[(&str, &str)] = &[
    ("À", "A"),
    ("Á", "A"),
    ("Â", "A"),
    ("Ã", "A"),
    ("Ä", "Ae"),
    ("Æ", "Ae"),
    ("Å", "Aa"),
    ("æ", "a"),
    ("Ç", "C"),
    ("È", "E"),
    ("É", "E"),
    ("Ê", "E"),
    ("Ë", "E"),
    ("Ì", "I"),
    ("Í", "I"),
    ("Î", "I"),
    ("Ï", "I"),
    ("Ñ", "N"),
    ("Ò", "O"),
    ("Ó", "O"),
    ("Ô", "O"),
    ("Õ", "O"),
    ("Ö", "Oe"),
    ("Ù", "U"),
    ("Ú", "U"),
    ("Û", "U"),
    ("Ü", "Ue"),
    ("Ý", "Y"),
    ("ß", "ss"),
    ("à", "a"),
    ("á", "a"),
    ("â", "a"),
    ("ã", "a"),
    ("ä", "ae"),
    ("å", "aa"),
    ("ç", "c"),
    ("è", "e"),
    ("é", "e"),
    ("ê", "e"),
    ("ë", "e"),
    ("ì", "i"),
    ("í", "i"),
    ("î", "i"),
    ("ï", "i"),
    ("ñ", "n"),
    ("ò", "o"),
    ("ó", "o"),
    ("ô", "o"),
    ("õ", "o"),
    ("ö", "oe"),
    ("ù", "u"),
    ("ú", "u"),
    ("û", "u"),
    ("ü", "ue"),
    ("ý", "y"),
    ("ÿ", "y"),
    ("Ā", "A"),
    ("ā", "a"),
    ("Ă", "A"),
    ("ă", "a"),
    ("Ą", "A"),
    ("ą", "a"),
    ("Ć", "C"),
    ("ć", "c"),
    ("Ĉ", "C"),
    ("ĉ", "c"),
    ("Ċ", "C"),
    ("ċ", "c"),
    ("Č", "C"),
    ("č", "c"),
    ("Ď", "D"),
    ("ď", "d"),
    ("Đ", "D"),
    ("đ", "d"),
    ("Ē", "E"),
    ("ē", "e"),
    ("Ĕ", "E"),
    ("ĕ", "e"),
    ("Ė", "E"),
    ("ė", "e"),
    ("Ę", "E"),
    ("ę", "e"),
    ("Ě", "E"),
    ("ě", "e"),
    ("Ĝ", "G"),
    ("ĝ", "g"),
    ("Ğ", "G"),
    ("ğ", "g"),
    ("Ġ", "G"),
    ("ġ", "g"),
    ("Ģ", "G"),
    ("ģ", "g"),
    ("Ĥ", "H"),
    ("ĥ", "h"),
    ("Ħ", "H"),
    ("ħ", "h"),
    ("Ĩ", "I"),
    ("ĩ", "i"),
    ("Ī", "I"),
    ("ī", "i"),
    ("Ĭ", "I"),
    ("ĭ", "i"),
    ("Į", "I"),
    ("į", "i"),
    ("İ", "I"),
    ("ı", "i"),
    ("Ĳ", "IJ"),
    ("ĳ", "ij"),
    ("Ĵ", "J"),
    ("ĵ", "j"),
    ("Ķ", "K"),
    ("ķ", "k"),
    ("ĸ", "k"),
    ("Ĺ", "L"),
    ("ĺ", "l"),
    ("Ļ", "L"),
    ("ļ", "l"),
    ("Ľ", "L"),
    ("ľ", "l"),
    ("Ŀ", "L"),
    ("ŀ", "l"),
    ("Ł", "L"),
    ("ł", "l"),
    ("Ń", "N"),
    ("ń", "n"),
    ("Ņ", "N"),
    ("ņ", "n"),
    ("Ň", "N"),
    ("ň", "n"),
    ("ŉ", "N"),
    ("Ŋ", "n"),
    ("ŋ", "N"),
    ("Ō", "O"),
    ("ō", "o"),
    ("Ŏ", "O"),
    ("ŏ", "o"),
    ("Ő", "O"),
    ("ő", "o"),
    ("Œ", "OE"),
    ("œ", "oe"),
    ("Ø", "O"),
    ("ø", "o"),
    ("Ŕ", "R"),
    ("ŕ", "r"),
    ("Ŗ", "R"),
    ("ŗ", "r"),
    ("Ř", "R"),
    ("ř", "r"),
    ("Ś", "S"),
    ("ś", "s"),
    ("Ŝ", "S"),
    ("ŝ", "s"),
    ("Ş", "S"),
    ("ş", "s"),
    ("Š", "S"),
    ("š", "s"),
    ("Ţ", "T"),
    ("ţ", "t"),
    ("Ť", "T"),
    ("ť", "t"),
    ("Ŧ", "T"),
    ("ŧ", "t"),
    ("Ũ", "U"),
    ("ũ", "u"),
    ("Ū", "U"),
    ("ū", "u"),
    ("Ŭ", "U"),
    ("ŭ", "u"),
    ("Ů", "U"),
    ("ů", "u"),
    ("Ű", "U"),
    ("ű", "u"),
    ("Ų", "U"),
    ("ų", "u"),
    ("Ŵ", "W"),
    ("ŵ", "w"),
    ("Ŷ", "Y"),
    ("ŷ", "y"),
    ("Ÿ", "Y"),
    ("Ź", "Z"),
    ("ź", "z"),
    ("Ż", "Z"),
    ("ż", "z"),
    ("Ž", "Z"),
    ("ž", "z"),
    ("ſ", "s"),
    ("€", "E"),
    ("£", ""),
];

/// `capitalize()`'s default delimiters.
pub const CAPITALIZE_DELIMITERS: &[u8] = b" \n\t\r\0\x0B-";

/// `Doctrine\Inflector\Inflector`.
#[derive(Clone)]
pub struct Inflector {
    singularizer: Arc<dyn WordInflector>,
    pluralizer: Arc<dyn WordInflector>,
}

/// A pattern of the methods below, compiled once.
fn regex(cell: &'static OnceLock<Option<Pcre>>, pattern: &[u8]) -> Option<&'static Pcre> {
    cell.get_or_init(|| Pcre::new(pattern).ok()).as_ref()
}

fn preg_replace_or_throw(
    regex: Option<&Pcre>,
    replacement: &[u8],
    value: &[u8],
) -> Result<Vec<u8>, InflectorError> {
    preg_replace(regex, replacement, value).ok_or_else(|| {
        let mut message = b"preg_replace returned null for value \"".to_vec();
        message.extend_from_slice(value);
        message.push(b'"');
        InflectorError::thrown("RuntimeException", message)
    })
}

impl Inflector {
    pub fn new(singularizer: Arc<dyn WordInflector>, pluralizer: Arc<dyn WordInflector>) -> Self {
        Self {
            singularizer,
            pluralizer,
        }
    }

    /// Converts a word into the format for a Doctrine table name. Converts `ModelName` to `model_name`.
    pub fn tableize(&self, word: &[u8]) -> Result<PhpString, InflectorError> {
        static TABLEIZE: OnceLock<Option<Pcre>> = OnceLock::new();
        let tableized =
            preg_replace_or_throw(regex(&TABLEIZE, br"~(?<=\w)([A-Z])~u"), b"_$1", word)?;

        Ok(PhpString::from(mb_strtolower(&tableized)))
    }

    /// Converts a word into the format for a Doctrine class name. Converts `table_name` to `TableName`.
    pub fn classify(&self, word: &[u8]) -> PhpString {
        PhpString::from(str_replace(
            &[b" ", b"_", b"-"],
            &[b"", b"", b""],
            &ucwords(word, b" _-"),
        ))
    }

    /// Camelizes a word. This uses the `classify()` method and turns the first character to lowercase.
    pub fn camelize(&self, word: &[u8]) -> PhpString {
        PhpString::from(lcfirst(self.classify(word).as_bytes()))
    }

    /// Uppercases words with configurable delimiters between words.
    ///
    /// Takes a string and capitalizes all of the words, like PHP's built-in
    /// ucwords function. This extends that behavior, however, by allowing the
    /// word delimiters to be configured, rather than only separating on
    /// whitespace; `None` takes [`CAPITALIZE_DELIMITERS`].
    pub fn capitalize(&self, string: &[u8], delimiters: Option<&[u8]>) -> PhpString {
        PhpString::from(ucwords(string, delimiters.unwrap_or(CAPITALIZE_DELIMITERS)))
    }

    /// Checks if the given string seems like it has utf8 characters in it.
    pub fn seems_utf8(&self, string: &[u8]) -> bool {
        let mut i = 0;
        while i < string.len() {
            let byte = string[i];
            if byte < 0x80 {
                i += 1;
                continue; // 0bbbbbbb
            }

            let n = if byte & 0xE0 == 0xC0 {
                1 // 110bbbbb
            } else if byte & 0xF0 == 0xE0 {
                2 // 1110bbbb
            } else if byte & 0xF8 == 0xF0 {
                3 // 11110bbb
            } else if byte & 0xFC == 0xF8 {
                4 // 111110bb
            } else if byte & 0xFE == 0xFC {
                5 // 1111110b
            } else {
                return false; // Does not match any model
            };

            for _ in 0..n {
                // n bytes matching 10bbbbbb follow ?
                i += 1;
                if i == string.len() || string[i] & 0xC0 != 0x80 {
                    return false;
                }
            }
            i += 1;
        }

        true
    }

    /// Remove any illegal characters, accents, etc.
    pub fn unaccent(&self, string: &[u8]) -> PhpString {
        if self.seems_utf8(string) {
            return PhpString::from(strtr_pairs(string, ACCENTED_CHARACTERS));
        }

        // Assume ISO-8859-1 if not UTF-8
        let characters_in: &[u8] = &[
            128, 131, 138, 142, 154, 158, 159, 162, 165, 181, 192, 193, 194, 195, 196, 197, 199,
            200, 201, 202, 203, 204, 205, 206, 207, 209, 210, 211, 212, 213, 214, 216, 217, 218,
            219, 220, 221, 224, 225, 226, 227, 228, 229, 231, 232, 233, 234, 235, 236, 237, 238,
            239, 241, 242, 243, 244, 245, 246, 248, 249, 250, 251, 252, 253, 255,
        ];
        let characters_out = b"EfSZszYcYuAAAAAACEEEEIIIINOOOOOOUUUUYaaaaaaceeeeiiiinoooooouuuuyy";

        let string = strtr_bytes(string, characters_in, characters_out);

        let double_chars_in: [&[u8]; 9] = [
            &[140],
            &[156],
            &[198],
            &[208],
            &[222],
            &[223],
            &[230],
            &[240],
            &[254],
        ];
        let double_chars_out: [&[u8]; 9] = [
            b"OE", b"oe", b"AE", b"DH", b"TH", b"ss", b"ae", b"dh", b"th",
        ];

        PhpString::from(str_replace(&double_chars_in, &double_chars_out, &string))
    }

    /// Convert any passed string to a url friendly string.
    /// Converts 'My first blog post' to 'my-first-blog-post'
    pub fn urlize(&self, string: &[u8]) -> Result<PhpString, InflectorError> {
        static PATTERNS: [OnceLock<Option<Pcre>>; 4] = [
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
        ];

        // Remove all non url friendly characters with the unaccent function
        let unaccented = self.unaccent(string);

        let lowered = mb_strtolower(unaccented.as_bytes());

        let replacements: [(&[u8], &[u8]); 4] = [
            (br"/\W/", b" "),
            (br"/([A-Z]+)([A-Z][a-z])/", br"\1_\2"),
            (br"/([a-z\d])([A-Z])/", br"\1_\2"),
            (br"/[^A-Z^a-z^0-9^\/]+/", b"-"),
        ];

        let mut urlized = lowered;

        for ((pattern, replacement), cell) in replacements.iter().zip(&PATTERNS) {
            urlized = preg_replace_or_throw(regex(cell, pattern), replacement, &urlized)?;
        }

        Ok(PhpString::from(trim_chars(&urlized, b"-")))
    }

    /// Returns a word in singular form.
    pub fn singularize(&self, word: &[u8]) -> PhpString {
        self.singularizer.inflect(word)
    }

    /// Returns a word in plural form.
    pub fn pluralize(&self, word: &[u8]) -> PhpString {
        self.pluralizer.inflect(word)
    }
}
