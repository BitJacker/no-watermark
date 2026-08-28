//! Result types produced by [`crate::analyze`].
//!
//! Everything here is machine readable and locale independent. Human facing
//! strings live in `nowm-i18n`; this crate only ever emits stable identifiers
//! so that the CLI, the GUI and third-party consumers can translate or render
//! them however they like.

use serde::{Deserialize, Serialize};

/// Family a suspicious character belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Unicode Tags block (U+E0000–U+E007F): a shadow ASCII alphabet that is
    /// invisible to humans but perfectly readable to a tokenizer.
    Tag,
    /// Zero-width and other characters that occupy no visual space.
    Invisible,
    /// Bidirectional formatting controls, which can reorder displayed text.
    Bidi,
    /// Variation selectors, the carrier of the "emoji smuggling" byte channel.
    VariationSelector,
    /// A non-Latin letter impersonating a Latin one inside a Latin word.
    Homoglyph,
    /// Whitespace that is not U+0020 but looks like it.
    Space,
    /// Curly quotes, dashes, ellipses and friends.
    Typography,
    /// Format characters Unicode itself has deprecated.
    Deprecated,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Tag,
        Category::Invisible,
        Category::Bidi,
        Category::VariationSelector,
        Category::Homoglyph,
        Category::Space,
        Category::Typography,
        Category::Deprecated,
    ];

    /// Stable snake_case identifier, also used as the i18n lookup key.
    pub fn key(self) -> &'static str {
        match self {
            Category::Tag => "tag",
            Category::Invisible => "invisible",
            Category::Bidi => "bidi",
            Category::VariationSelector => "variation_selector",
            Category::Homoglyph => "homoglyph",
            Category::Space => "space",
            Category::Typography => "typography",
            Category::Deprecated => "deprecated",
        }
    }

    pub fn severity(self) -> Severity {
        match self {
            Category::Tag => Severity::Critical,
            Category::Invisible | Category::Bidi => Severity::High,
            Category::VariationSelector | Category::Homoglyph => Severity::Medium,
            Category::Deprecated => Severity::Medium,
            Category::Space => Severity::Low,
            Category::Typography => Severity::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn key(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }
}

/// What the cleaner did with a given character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Action {
    /// Deleted from the output.
    Removed,
    /// Substituted with the contained text.
    Replaced(String),
    /// Reported but left in place, either because the profile does not touch
    /// this category or because the character is legitimate here.
    Kept,
}

/// Machine-readable explanation attached to a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Note {
    /// A ZERO WIDTH JOINER holding a real emoji sequence together.
    LegitimateEmojiJoiner,
    /// A joiner doing orthographic work in Arabic, Persian or an Indic script.
    LegitimateScriptJoiner,
    /// Byte-order mark at the very start of the input.
    ByteOrderMark,
    /// Part of a decoded hidden payload (see [`Report::payloads`]).
    PartOfPayload,
    /// A narrow no-break space directly next to an em dash: the artefact that
    /// was widely (and wrongly) reported as a ChatGPT watermark.
    NarrowSpaceAroundDash,
}

impl Note {
    pub fn key(self) -> &'static str {
        match self {
            Note::LegitimateEmojiJoiner => "legitimate_emoji_joiner",
            Note::LegitimateScriptJoiner => "legitimate_script_joiner",
            Note::ByteOrderMark => "byte_order_mark",
            Note::PartOfPayload => "part_of_payload",
            Note::NarrowSpaceAroundDash => "narrow_space_around_dash",
        }
    }
}

/// One suspicious character, located precisely in the input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Index in `char`s from the start of the input.
    pub char_index: usize,
    /// Index in bytes from the start of the input.
    pub byte_offset: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column, counted in `char`s.
    pub column: usize,
    pub codepoint: u32,
    /// `U+200B` style rendering of `codepoint`.
    pub display: String,
    /// Canonical Unicode name, or `CONFUSABLE LETTER` for homoglyphs.
    pub name: String,
    pub category: Category,
    pub severity: Severity,
    pub action: Action,
    pub note: Option<Note>,
}

/// How a hidden payload was encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadKind {
    /// Text mirrored into the Unicode Tags block.
    UnicodeTags,
    /// Bytes encoded as a run of variation selectors.
    VariationSelector,
    /// Bits encoded as a run of two distinct zero-width characters.
    ZeroWidthBinary,
}

impl PayloadKind {
    pub fn key(self) -> &'static str {
        match self {
            PayloadKind::UnicodeTags => "unicode_tags",
            PayloadKind::VariationSelector => "variation_selector",
            PayloadKind::ZeroWidthBinary => "zero_width_binary",
        }
    }
}

/// Content that was hidden inside the input and has been recovered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenPayload {
    pub kind: PayloadKind,
    /// Decoded text, when the bytes formed valid UTF-8.
    pub text: Option<String>,
    /// Raw decoded bytes, lowercase hex. Always present.
    pub hex: String,
    /// Character index of the first carrier character.
    pub start_char: usize,
    /// Number of carrier characters consumed.
    pub len_chars: usize,
}

/// A stylometric observation. Never proof of anything on its own — these are
/// reported so a human can judge, not so the tool can accuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    /// Narrow no-break space adjacent to an em dash.
    NarrowSpaceEmDash,
    /// Unusually high em-dash density.
    EmDashDensity,
    /// Typographic (curly) quotes used throughout.
    CurlyQuotes,
    /// A single-character ellipsis instead of three dots.
    UnicodeEllipsis,
    /// Non-breaking spaces sprinkled through ordinary prose.
    NonBreakingSpaces,
}

impl SignalKind {
    pub fn key(self) -> &'static str {
        match self {
            SignalKind::NarrowSpaceEmDash => "narrow_space_em_dash",
            SignalKind::EmDashDensity => "em_dash_density",
            SignalKind::CurlyQuotes => "curly_quotes",
            SignalKind::UnicodeEllipsis => "unicode_ellipsis",
            SignalKind::NonBreakingSpaces => "non_breaking_spaces",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signal {
    pub kind: SignalKind,
    pub count: usize,
    /// Occurrences per 1000 characters, scaled by 100 to stay integral.
    pub per_1k_x100: u32,
}

/// Overall judgement about the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Nothing worth acting on.
    Clean,
    /// Cosmetic oddities only: exotic spaces, curly punctuation.
    Cosmetic,
    /// Invisible or deceptive characters present.
    Suspicious,
    /// Readable content was hidden inside the text.
    HiddenContent,
}

impl Verdict {
    pub fn key(self) -> &'static str {
        match self {
            Verdict::Clean => "clean",
            Verdict::Cosmetic => "cosmetic",
            Verdict::Suspicious => "suspicious",
            Verdict::HiddenContent => "hidden_content",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub input_chars: usize,
    pub input_bytes: usize,
    pub output_chars: usize,
    pub output_bytes: usize,
    pub removed: usize,
    pub replaced: usize,
    pub kept: usize,
    /// Per-category counts, ordered by [`Category::ALL`].
    pub by_category: Vec<(Category, usize)>,
}

/// Full result of an analysis run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// The sanitised text. Equal to the input when the profile is inert.
    pub cleaned: String,
    pub findings: Vec<Finding>,
    pub payloads: Vec<HiddenPayload>,
    pub signals: Vec<Signal>,
    pub stats: Stats,
    /// 0–100. Not a probability: a triage aid.
    pub score: u8,
    pub verdict: Verdict,
}

impl Report {
    /// True when the cleaner changed anything.
    pub fn modified(&self) -> bool {
        self.stats.removed > 0 || self.stats.replaced > 0
    }

    /// True when anything at or above `min` was found.
    pub fn has_severity(&self, min: Severity) -> bool {
        self.findings.iter().any(|f| f.severity >= min)
    }

    pub fn count_of(&self, category: Category) -> usize {
        self.stats
            .by_category
            .iter()
            .find(|(c, _)| *c == category)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    }
}
