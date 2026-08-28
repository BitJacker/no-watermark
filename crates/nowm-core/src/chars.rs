//! Unicode character classification tables.
//!
//! Every codepoint that no-watermark knows about is described here exactly
//! once. The tables are deliberately hand written rather than derived from the
//! Unicode database: the interesting set is small, and being explicit makes it
//! possible to attach a rationale (and a safe replacement) to every entry.

use crate::report::Category;

/// What no-watermark knows about a single suspicious codepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharInfo {
    /// Canonical Unicode name, used verbatim in reports (names are not
    /// translated: they are identifiers, not prose).
    pub name: &'static str,
    pub category: Category,
    /// Text this character folds to when the active profile normalises it.
    /// `None` means "delete outright".
    pub replacement: Option<&'static str>,
}

const fn info(
    name: &'static str,
    category: Category,
    replacement: Option<&'static str>,
) -> CharInfo {
    CharInfo {
        name,
        category,
        replacement,
    }
}

/// Classify a character. Returns `None` for ordinary text.
pub fn classify(ch: char) -> Option<CharInfo> {
    let cp = ch as u32;

    // Unicode Tags block: a full shadow copy of printable ASCII that renders
    // as absolutely nothing. There is no legitimate use in chat text.
    if (0xE0000..=0xE007F).contains(&cp) {
        return Some(info("TAG CHARACTER", Category::Tag, None));
    }
    // Supplementary variation selectors VS17..VS256.
    if (0xE0100..=0xE01EF).contains(&cp) {
        return Some(info(
            "VARIATION SELECTOR (SUPPLEMENT)",
            Category::VariationSelector,
            None,
        ));
    }
    // Musical format controls: invisible, and valid only inside musical notation.
    if (0x1D173..=0x1D17A).contains(&cp) {
        return Some(info(
            "MUSICAL SYMBOL FORMAT CONTROL",
            Category::Invisible,
            None,
        ));
    }

    let i = match cp {
        // ---- invisible / zero-width -------------------------------------
        0x00AD => info("SOFT HYPHEN", Category::Invisible, None),
        0x034F => info("COMBINING GRAPHEME JOINER", Category::Invisible, None),
        0x115F => info("HANGUL CHOSEONG FILLER", Category::Invisible, None),
        0x1160 => info("HANGUL JUNGSEONG FILLER", Category::Invisible, None),
        0x17B4 => info("KHMER VOWEL INHERENT AQ", Category::Invisible, None),
        0x17B5 => info("KHMER VOWEL INHERENT AA", Category::Invisible, None),
        0x180E => info("MONGOLIAN VOWEL SEPARATOR", Category::Invisible, None),
        0x200B => info("ZERO WIDTH SPACE", Category::Invisible, None),
        0x200C => info("ZERO WIDTH NON-JOINER", Category::Invisible, None),
        0x200D => info("ZERO WIDTH JOINER", Category::Invisible, None),
        0x2060 => info("WORD JOINER", Category::Invisible, None),
        0x2061 => info("FUNCTION APPLICATION", Category::Invisible, None),
        0x2062 => info("INVISIBLE TIMES", Category::Invisible, None),
        0x2063 => info("INVISIBLE SEPARATOR", Category::Invisible, None),
        0x2064 => info("INVISIBLE PLUS", Category::Invisible, None),
        0x2065 => info("UNASSIGNED FORMAT CHARACTER", Category::Invisible, None),
        0x206A => info("INHIBIT SYMMETRIC SWAPPING", Category::Deprecated, None),
        0x206B => info("ACTIVATE SYMMETRIC SWAPPING", Category::Deprecated, None),
        0x206C => info("INHIBIT ARABIC FORM SHAPING", Category::Deprecated, None),
        0x206D => info("ACTIVATE ARABIC FORM SHAPING", Category::Deprecated, None),
        0x206E => info("NATIONAL DIGIT SHAPES", Category::Deprecated, None),
        0x206F => info("NOMINAL DIGIT SHAPES", Category::Deprecated, None),
        0x3164 => info("HANGUL FILLER", Category::Invisible, None),
        0xFEFF => info("ZERO WIDTH NO-BREAK SPACE (BOM)", Category::Invisible, None),
        0xFFA0 => info("HALFWIDTH HANGUL FILLER", Category::Invisible, None),
        0xFFF9 => info("INTERLINEAR ANNOTATION ANCHOR", Category::Invisible, None),
        0xFFFA => info(
            "INTERLINEAR ANNOTATION SEPARATOR",
            Category::Invisible,
            None,
        ),
        0xFFFB => info(
            "INTERLINEAR ANNOTATION TERMINATOR",
            Category::Invisible,
            None,
        ),

        // ---- bidirectional controls --------------------------------------
        0x061C => info("ARABIC LETTER MARK", Category::Bidi, None),
        0x200E => info("LEFT-TO-RIGHT MARK", Category::Bidi, None),
        0x200F => info("RIGHT-TO-LEFT MARK", Category::Bidi, None),
        0x202A => info("LEFT-TO-RIGHT EMBEDDING", Category::Bidi, None),
        0x202B => info("RIGHT-TO-LEFT EMBEDDING", Category::Bidi, None),
        0x202C => info("POP DIRECTIONAL FORMATTING", Category::Bidi, None),
        0x202D => info("LEFT-TO-RIGHT OVERRIDE", Category::Bidi, None),
        0x202E => info("RIGHT-TO-LEFT OVERRIDE", Category::Bidi, None),
        0x2066 => info("LEFT-TO-RIGHT ISOLATE", Category::Bidi, None),
        0x2067 => info("RIGHT-TO-LEFT ISOLATE", Category::Bidi, None),
        0x2068 => info("FIRST STRONG ISOLATE", Category::Bidi, None),
        0x2069 => info("POP DIRECTIONAL ISOLATE", Category::Bidi, None),

        // ---- variation selectors ------------------------------------------
        0x180B => info(
            "MONGOLIAN FREE VARIATION SELECTOR ONE",
            Category::VariationSelector,
            None,
        ),
        0x180C => info(
            "MONGOLIAN FREE VARIATION SELECTOR TWO",
            Category::VariationSelector,
            None,
        ),
        0x180D => info(
            "MONGOLIAN FREE VARIATION SELECTOR THREE",
            Category::VariationSelector,
            None,
        ),
        0xFE00..=0xFE0F => info("VARIATION SELECTOR", Category::VariationSelector, None),

        // ---- whitespace look-alikes ---------------------------------------
        0x00A0 => info("NO-BREAK SPACE", Category::Space, Some(" ")),
        0x1680 => info("OGHAM SPACE MARK", Category::Space, Some(" ")),
        0x2000 => info("EN QUAD", Category::Space, Some(" ")),
        0x2001 => info("EM QUAD", Category::Space, Some(" ")),
        0x2002 => info("EN SPACE", Category::Space, Some(" ")),
        0x2003 => info("EM SPACE", Category::Space, Some(" ")),
        0x2004 => info("THREE-PER-EM SPACE", Category::Space, Some(" ")),
        0x2005 => info("FOUR-PER-EM SPACE", Category::Space, Some(" ")),
        0x2006 => info("SIX-PER-EM SPACE", Category::Space, Some(" ")),
        0x2007 => info("FIGURE SPACE", Category::Space, Some(" ")),
        0x2008 => info("PUNCTUATION SPACE", Category::Space, Some(" ")),
        0x2009 => info("THIN SPACE", Category::Space, Some(" ")),
        0x200A => info("HAIR SPACE", Category::Space, Some(" ")),
        0x202F => info("NARROW NO-BREAK SPACE", Category::Space, Some(" ")),
        0x205F => info("MEDIUM MATHEMATICAL SPACE", Category::Space, Some(" ")),
        0x3000 => info("IDEOGRAPHIC SPACE", Category::Space, Some(" ")),
        0x2800 => info("BRAILLE PATTERN BLANK", Category::Space, Some(" ")),

        // ---- typography ----------------------------------------------------
        0x2010 => info("HYPHEN", Category::Typography, Some("-")),
        0x2011 => info("NON-BREAKING HYPHEN", Category::Typography, Some("-")),
        0x2012 => info("FIGURE DASH", Category::Typography, Some("-")),
        0x2013 => info("EN DASH", Category::Typography, Some("-")),
        0x2014 => info("EM DASH", Category::Typography, Some("-")),
        0x2015 => info("HORIZONTAL BAR", Category::Typography, Some("-")),
        0x2212 => info("MINUS SIGN", Category::Typography, Some("-")),
        0x2018 => info(
            "LEFT SINGLE QUOTATION MARK",
            Category::Typography,
            Some("'"),
        ),
        0x2019 => info(
            "RIGHT SINGLE QUOTATION MARK",
            Category::Typography,
            Some("'"),
        ),
        0x201A => info(
            "SINGLE LOW-9 QUOTATION MARK",
            Category::Typography,
            Some("'"),
        ),
        0x201B => info(
            "SINGLE HIGH-REVERSED-9 QUOTATION MARK",
            Category::Typography,
            Some("'"),
        ),
        0x201C => info(
            "LEFT DOUBLE QUOTATION MARK",
            Category::Typography,
            Some("\""),
        ),
        0x201D => info(
            "RIGHT DOUBLE QUOTATION MARK",
            Category::Typography,
            Some("\""),
        ),
        0x201E => info(
            "DOUBLE LOW-9 QUOTATION MARK",
            Category::Typography,
            Some("\""),
        ),
        0x201F => info(
            "DOUBLE HIGH-REVERSED-9 QUOTATION MARK",
            Category::Typography,
            Some("\""),
        ),
        0x2032 => info("PRIME", Category::Typography, Some("'")),
        0x2033 => info("DOUBLE PRIME", Category::Typography, Some("\"")),
        0x02BC => info(
            "MODIFIER LETTER APOSTROPHE",
            Category::Typography,
            Some("'"),
        ),
        0x2026 => info("HORIZONTAL ELLIPSIS", Category::Typography, Some("...")),
        0x2044 => info("FRACTION SLASH", Category::Typography, Some("/")),

        _ => return None,
    };
    Some(i)
}

/// Characters that carry a byte of payload in the variation-selector
/// steganography scheme popularised for "emoji smuggling".
pub fn variation_selector_byte(ch: char) -> Option<u8> {
    let cp = ch as u32;
    match cp {
        0xFE00..=0xFE0F => Some((cp - 0xFE00) as u8),
        0xE0100..=0xE01EF => Some((cp - 0xE0100 + 16) as u8),
        _ => None,
    }
}

/// Decode a Tags-block codepoint back to the ASCII character it mirrors.
pub fn tag_char_ascii(ch: char) -> Option<char> {
    let cp = ch as u32;
    if (0xE0000..=0xE007F).contains(&cp) {
        let ascii = (cp - 0xE0000) as u8;
        // U+E0001 is LANGUAGE TAG and U+E007F is CANCEL TAG: framing, not data.
        if ascii == 0x01 || ascii == 0x7F {
            return None;
        }
        if ascii.is_ascii() {
            return Some(ascii as char);
        }
    }
    None
}

/// Rough `Extended_Pictographic` test, good enough to recognise an emoji
/// sequence and therefore to protect a legitimate ZERO WIDTH JOINER.
pub fn is_pictographic(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        0x203C | 0x2049 | 0x2122 | 0x2139 | 0x3030 | 0x303D | 0x3297 | 0x3299
        | 0x2194..=0x21AA
        | 0x231A..=0x231B
        | 0x2328
        | 0x23CF..=0x23FA
        | 0x24C2
        | 0x25AA..=0x25FE
        | 0x2600..=0x27EF
        | 0x2934..=0x2935
        | 0x2B00..=0x2BFF
        | 0x1F000..=0x1FAFF
        | 0x1FC00..=0x1FFFD
    )
}

/// Emoji modifiers that may sit between a base emoji and a joiner.
pub fn is_emoji_modifier(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp, 0xFE0F | 0xFE0E | 0x1F3FB..=0x1F3FF | 0x20E3)
}

/// Scripts in which ZERO WIDTH NON-JOINER / JOINER carry orthographic meaning.
/// Stripping them there would corrupt real Persian, Arabic or Indic text.
pub fn is_joining_script(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        0x0590..=0x05FF   // Hebrew
        | 0x0600..=0x06FF // Arabic
        | 0x0700..=0x074F // Syriac
        | 0x0750..=0x077F // Arabic Supplement
        | 0x0780..=0x07BF // Thaana
        | 0x0900..=0x0DFF // Indic scripts
        | 0x0E00..=0x0E7F // Thai / Lao
        | 0x1800..=0x18AF // Mongolian
        | 0xFB1D..=0xFDFF // Hebrew / Arabic presentation forms
        | 0xFE70..=0xFEFC // Arabic presentation forms-B
    )
}

/// True for the ASCII characters an AI answer is *expected* to contain.
pub fn is_expected_ascii_control(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\t')
}
