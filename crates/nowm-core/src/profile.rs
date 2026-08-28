//! Cleaning profiles.
//!
//! Three presets cover almost every real use, and each one is a strict
//! superset of the previous. `Safe` is the important one: it is guaranteed
//! not to change a single visible glyph.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// Report only; the output is byte-identical to the input.
    Scan,
    /// Remove what is invisible or deceptive. Visible text is untouched.
    Safe,
    /// Safe, plus exotic whitespace folded to U+0020 and homoglyphs folded to
    /// ASCII.
    Standard,
    /// Standard, plus NFKC normalisation and ASCII typography.
    Aggressive,
}

impl Preset {
    pub fn key(self) -> &'static str {
        match self {
            Preset::Scan => "scan",
            Preset::Safe => "safe",
            Preset::Standard => "standard",
            Preset::Aggressive => "aggressive",
        }
    }

    pub const ALL: [Preset; 4] = [
        Preset::Scan,
        Preset::Safe,
        Preset::Standard,
        Preset::Aggressive,
    ];
}

/// Exactly what the cleaner is allowed to touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Strip zero-width and other invisible format characters.
    pub remove_invisible: bool,
    /// Strip bidirectional formatting controls.
    pub remove_bidi: bool,
    /// Strip Unicode Tags-block characters.
    pub remove_tags: bool,
    /// Strip variation selectors.
    pub remove_variation_selectors: bool,
    /// Fold NBSP, thin space, narrow no-break space, … to U+0020.
    pub normalize_spaces: bool,
    /// Fold confusable letters to ASCII inside Latin words.
    pub fold_homoglyphs: bool,
    /// Fold curly quotes, dashes and ellipses to ASCII.
    pub ascii_typography: bool,
    /// Apply Unicode NFKC normalisation to the result.
    pub nfkc: bool,
    /// Keep a ZERO WIDTH JOINER that is holding an emoji sequence together.
    pub preserve_emoji_joiners: bool,
    /// Keep joiners that carry orthographic meaning in Arabic or Indic scripts.
    pub preserve_script_joiners: bool,
    /// Drop trailing spaces and tabs at the end of every line.
    pub trim_trailing_whitespace: bool,
    /// Collapse three or more consecutive blank lines into one.
    pub collapse_blank_lines: bool,
    /// Rewrite every line ending to this sequence.
    pub line_ending: Option<LineEnding>,
}

impl Profile {
    /// Report-only: nothing is altered.
    pub fn scan() -> Self {
        Profile {
            remove_invisible: false,
            remove_bidi: false,
            remove_tags: false,
            remove_variation_selectors: false,
            normalize_spaces: false,
            fold_homoglyphs: false,
            ascii_typography: false,
            nfkc: false,
            preserve_emoji_joiners: true,
            preserve_script_joiners: true,
            trim_trailing_whitespace: false,
            collapse_blank_lines: false,
            line_ending: None,
        }
    }

    /// Removes only what a human cannot see. Visible glyphs are preserved
    /// exactly, so the cleaned text is safe to paste back anywhere.
    pub fn safe() -> Self {
        Profile {
            remove_invisible: true,
            remove_bidi: true,
            remove_tags: true,
            remove_variation_selectors: true,
            ..Profile::scan()
        }
    }

    /// The recommended default: invisibles gone, whitespace and confusables
    /// normalised, punctuation left as the author wrote it.
    pub fn standard() -> Self {
        Profile {
            normalize_spaces: true,
            fold_homoglyphs: true,
            trim_trailing_whitespace: true,
            ..Profile::safe()
        }
    }

    /// Everything, including ASCII-only punctuation. Changes how the text
    /// looks, so it is opt-in.
    pub fn aggressive() -> Self {
        Profile {
            ascii_typography: true,
            nfkc: true,
            collapse_blank_lines: true,
            ..Profile::standard()
        }
    }

    pub fn from_preset(preset: Preset) -> Self {
        match preset {
            Preset::Scan => Profile::scan(),
            Preset::Safe => Profile::safe(),
            Preset::Standard => Profile::standard(),
            Preset::Aggressive => Profile::aggressive(),
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Profile::standard()
    }
}
