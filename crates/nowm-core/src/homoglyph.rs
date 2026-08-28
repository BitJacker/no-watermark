//! Confusable letters that impersonate ASCII inside Latin words.
//!
//! Only characters that Unicode NFKC normalisation leaves alone are listed
//! here. Fullwidth forms, mathematical alphanumerics and Roman numerals are
//! deliberately absent: NFKC already folds those, so duplicating them would
//! only create two ways to be wrong.

/// The ASCII letter this character is trying to look like, if any.
pub fn target(ch: char) -> Option<char> {
    let t = match ch {
        // ---- Cyrillic ----------------------------------------------------
        '\u{0405}' => 'S',
        '\u{0406}' => 'I',
        '\u{0408}' => 'J',
        '\u{0410}' => 'A',
        '\u{0412}' => 'B',
        '\u{0415}' => 'E',
        '\u{041A}' => 'K',
        '\u{041C}' => 'M',
        '\u{041D}' => 'H',
        '\u{041E}' => 'O',
        '\u{0420}' => 'P',
        '\u{0421}' => 'C',
        '\u{0422}' => 'T',
        '\u{0423}' => 'Y',
        '\u{0425}' => 'X',
        '\u{0430}' => 'a',
        '\u{0435}' => 'e',
        '\u{043E}' => 'o',
        '\u{0440}' => 'p',
        '\u{0441}' => 'c',
        '\u{0443}' => 'y',
        '\u{0445}' => 'x',
        '\u{0455}' => 's',
        '\u{0456}' => 'i',
        '\u{0458}' => 'j',
        '\u{0501}' => 'd',
        '\u{051A}' => 'Q',
        '\u{051B}' => 'q',
        '\u{051C}' => 'W',
        '\u{051D}' => 'w',
        '\u{04AE}' => 'Y',
        '\u{04BB}' => 'h',
        '\u{04C0}' => 'I',
        '\u{04CF}' => 'l',

        // ---- Greek --------------------------------------------------------
        '\u{0391}' => 'A',
        '\u{0392}' => 'B',
        '\u{0395}' => 'E',
        '\u{0396}' => 'Z',
        '\u{0397}' => 'H',
        '\u{0399}' => 'I',
        '\u{039A}' => 'K',
        '\u{039C}' => 'M',
        '\u{039D}' => 'N',
        '\u{039F}' => 'O',
        '\u{03A1}' => 'P',
        '\u{03A4}' => 'T',
        '\u{03A5}' => 'Y',
        '\u{03A7}' => 'X',
        '\u{03BF}' => 'o',
        '\u{03C1}' => 'p',
        '\u{03BD}' => 'v',
        '\u{03F2}' => 'c',
        '\u{03F3}' => 'j',

        // ---- Armenian -----------------------------------------------------
        '\u{0585}' => 'o',
        '\u{0578}' => 'n',

        // ---- Cherokee -----------------------------------------------------
        '\u{13A0}' => 'D',
        '\u{13A2}' => 'T',
        '\u{13AA}' => 'A',
        '\u{13AC}' => 'E',
        '\u{13B7}' => 'M',
        '\u{13BB}' => 'H',
        '\u{13BF}' => 'C',
        '\u{13C3}' => 'Z',
        '\u{13D2}' => 'R',
        '\u{13D4}' => 'W',
        '\u{13DE}' => 'L',
        '\u{13E2}' => 'P',
        '\u{13E6}' => 'K',
        '\u{13F4}' => 'B',

        // ---- Latin extensions that are not NFKC-equivalent ----------------
        '\u{0131}' => 'i', // DOTLESS I
        '\u{0251}' => 'a', // LATIN SMALL LETTER ALPHA
        '\u{0261}' => 'g', // LATIN SMALL LETTER SCRIPT G
        '\u{1D0F}' => 'o', // LATIN LETTER SMALL CAPITAL O
        '\u{01C0}' => 'l', // LATIN LETTER DENTAL CLICK

        _ => return None,
    };
    Some(t)
}

/// A character that can take part in a word for the purposes of homoglyph
/// context detection.
pub fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '\'' || ch == '-'
}

/// Decide, for every character index in `text`, whether a confusable should be
/// folded to ASCII.
///
/// The rule is contextual on purpose: a Cyrillic `о` is only a homoglyph if it
/// sits in a word that is otherwise Latin. Folding unconditionally would
/// silently mangle genuine Russian, Greek or Cherokee text — which is a far
/// worse outcome than leaving one impersonating letter in place.
pub fn plan(text: &str) -> Vec<(usize, char)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < chars.len() {
        if !is_word_char(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && is_word_char(chars[i]) {
            i += 1;
        }
        let word = &chars[start..i];

        let mut ascii_letters = 0usize;
        let mut confusables: Vec<(usize, char)> = Vec::new();
        let mut foreign_non_confusable = 0usize;

        for (offset, &ch) in word.iter().enumerate() {
            if ch.is_ascii_alphanumeric() {
                ascii_letters += 1;
            } else if let Some(t) = target(ch) {
                confusables.push((start + offset, t));
            } else if !ch.is_ascii() {
                foreign_non_confusable += 1;
            }
        }

        // Mixed-script word with a Latin majority and no unrelated foreign
        // letters: the confusables are impersonating ASCII.
        if !confusables.is_empty() && ascii_letters > 0 && foreign_non_confusable == 0 {
            out.extend(confusables);
        }
    }

    out
}
