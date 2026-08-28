//! # nowm-core
//!
//! Detection and removal engine behind **no-watermark**.
//!
//! ## What this crate does
//!
//! It finds, explains and strips the character-level fingerprints that end up
//! in text copied out of an AI chat:
//!
//! * **Unicode Tags block** (`U+E0000`–`U+E007F`) — a shadow ASCII alphabet
//!   that renders as nothing at all yet is read normally by a tokenizer. This
//!   is the carrier used for *ASCII smuggling* / invisible prompt injection.
//! * **Zero-width and format characters** — `ZWSP`, `ZWNJ`, `ZWJ`, word
//!   joiner, soft hyphen, BOM, Hangul fillers, and friends.
//! * **Variation selectors** — the byte channel behind "emoji smuggling".
//! * **Bidirectional controls** — which can make displayed text differ from
//!   stored text.
//! * **Homoglyphs** — Cyrillic `о` standing in for Latin `o`.
//! * **Exotic whitespace** — including `U+202F NARROW NO-BREAK SPACE`, the
//!   character that some ChatGPT models emitted in 2025 and that was widely
//!   (and incorrectly) reported as an official watermark.
//!
//! ## What this crate does *not* do
//!
//! It cannot remove a **statistical watermark** such as Google DeepMind's
//! SynthID-Text, which is embedded in the model's token sampling rather than
//! in any particular character. No character-level tool can. Say so plainly
//! rather than implying otherwise.
//!
//! ## Example
//!
//! ```
//! use nowm_core::{analyze, Profile};
//!
//! let dirty = "Hello\u{200B} world";
//! let report = analyze(dirty, &Profile::safe());
//! assert_eq!(report.cleaned, "Hello world");
//! assert_eq!(report.findings.len(), 1);
//! ```

pub mod chars;
pub mod homoglyph;
pub mod payload;
pub mod profile;
pub mod report;

use std::collections::BTreeMap;

use unicode_normalization::UnicodeNormalization;

use crate::chars::{classify, is_emoji_modifier, is_joining_script, is_pictographic};

pub use crate::profile::{LineEnding, Preset, Profile};
pub use crate::report::{
    Action, Category, Finding, HiddenPayload, Note, PayloadKind, Report, Severity, Signal,
    SignalKind, Stats, Verdict,
};

/// Name reported for a confusable letter. Unicode has no single name for the
/// concept, so no-watermark defines one.
const CONFUSABLE_NAME: &str = "CONFUSABLE LETTER";

/// Analyse `text` and produce both a report and the cleaned output.
pub fn analyze(text: &str, profile: &Profile) -> Report {
    let chars: Vec<char> = text.chars().collect();
    let scan = payload::scan(&chars);

    let homoglyph_plan: BTreeMap<usize, char> = homoglyph::plan(text).into_iter().collect();

    let mut out = String::with_capacity(text.len());
    let mut findings: Vec<Finding> = Vec::new();
    let mut line = 1usize;
    let mut column = 1usize;

    for (char_index, (byte_offset, ch)) in text.char_indices().enumerate() {
        let mut handled = false;

        if let Some(&target) = homoglyph_plan.get(&char_index) {
            let action = if profile.fold_homoglyphs {
                out.push(target);
                Action::Replaced(target.to_string())
            } else {
                out.push(ch);
                Action::Kept
            };
            findings.push(make_finding(
                char_index,
                byte_offset,
                line,
                column,
                ch,
                CONFUSABLE_NAME,
                Category::Homoglyph,
                action,
                None,
            ));
            handled = true;
        } else if let Some(info) = classify(ch) {
            let note = note_for(&chars, char_index, ch, &scan);
            let keep = should_keep(profile, info.category, note);

            let action = if keep {
                out.push(ch);
                Action::Kept
            } else if let Some(rep) = info.replacement {
                out.push_str(rep);
                Action::Replaced(rep.to_string())
            } else {
                Action::Removed
            };

            findings.push(make_finding(
                char_index,
                byte_offset,
                line,
                column,
                ch,
                info.name,
                info.category,
                action,
                note,
            ));
            handled = true;
        }

        if !handled {
            out.push(ch);
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    let cleaned = post_process(out, profile);
    let signals = signals_for(&chars);
    let stats = stats_for(text, &cleaned, &findings);
    let (score, verdict) = score_for(&findings, &scan.payloads);

    Report {
        cleaned,
        findings,
        payloads: scan.payloads,
        signals,
        stats,
        score,
        verdict,
    }
}

/// Convenience wrapper when only the cleaned text is needed.
pub fn clean(text: &str, profile: &Profile) -> String {
    analyze(text, profile).cleaned
}

/// Render `text` with every suspicious character made visible, for previews.
///
/// Invisible characters become `⟦U+200B⟧`, whitespace look-alikes become `␣`
/// and confusables are wrapped in `⟨ ⟩`.
pub fn visualize(text: &str) -> String {
    let homoglyph_plan: BTreeMap<usize, char> = homoglyph::plan(text).into_iter().collect();
    let mut out = String::with_capacity(text.len());

    for (char_index, ch) in text.chars().enumerate() {
        if homoglyph_plan.contains_key(&char_index) {
            out.push('⟨');
            out.push(ch);
            out.push('⟩');
            continue;
        }
        match classify(ch) {
            Some(info) if info.category == Category::Space => out.push('␣'),
            Some(info) if info.category == Category::Typography => out.push(ch),
            Some(_) => {
                out.push_str(&format!("⟦{}⟧", codepoint_label(ch)));
            }
            None => out.push(ch),
        }
    }
    out
}

/// `U+200B` style label for a character.
pub fn codepoint_label(ch: char) -> String {
    format!("U+{:04X}", ch as u32)
}

// ---------------------------------------------------------------------------
// internals
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn make_finding(
    char_index: usize,
    byte_offset: usize,
    line: usize,
    column: usize,
    ch: char,
    name: &'static str,
    category: Category,
    action: Action,
    note: Option<Note>,
) -> Finding {
    Finding {
        char_index,
        byte_offset,
        line,
        column,
        codepoint: ch as u32,
        display: codepoint_label(ch),
        name: name.to_string(),
        category,
        severity: category.severity(),
        action,
        note,
    }
}

/// Attach the reason a character is (or is not) legitimate in this position.
fn note_for(chars: &[char], index: usize, ch: char, scan: &payload::PayloadScan) -> Option<Note> {
    if scan.carrier_indices.contains(&index) {
        return Some(Note::PartOfPayload);
    }
    match ch {
        '\u{200D}' if emoji_joiner_context(chars, index) => Some(Note::LegitimateEmojiJoiner),
        '\u{200D}' | '\u{200C}' if script_joiner_context(chars, index) => {
            Some(Note::LegitimateScriptJoiner)
        }
        '\u{FE0F}' | '\u{FE0E}' if index > 0 && preceded_by_pictograph(chars, index) => {
            Some(Note::LegitimateEmojiJoiner)
        }
        '\u{FEFF}' if index == 0 => Some(Note::ByteOrderMark),
        '\u{202F}' if adjacent_to_dash(chars, index) => Some(Note::NarrowSpaceAroundDash),
        _ => None,
    }
}

/// Whether the profile leaves this character alone.
fn should_keep(profile: &Profile, category: Category, note: Option<Note>) -> bool {
    // A payload carrier is never "legitimate", whatever it looks like.
    if note == Some(Note::PartOfPayload) {
        return !enabled_for(profile, category);
    }
    match note {
        Some(Note::LegitimateEmojiJoiner) if profile.preserve_emoji_joiners => return true,
        Some(Note::LegitimateScriptJoiner) if profile.preserve_script_joiners => return true,
        _ => {}
    }
    !enabled_for(profile, category)
}

fn enabled_for(profile: &Profile, category: Category) -> bool {
    match category {
        Category::Tag => profile.remove_tags,
        Category::Invisible | Category::Deprecated => profile.remove_invisible,
        Category::Bidi => profile.remove_bidi,
        Category::VariationSelector => profile.remove_variation_selectors,
        Category::Space => profile.normalize_spaces,
        Category::Typography => profile.ascii_typography,
        Category::Homoglyph => profile.fold_homoglyphs,
    }
}

fn neighbour(chars: &[char], from: usize, forward: bool) -> Option<char> {
    let mut i = from;
    loop {
        i = if forward {
            i.checked_add(1)?
        } else {
            i.checked_sub(1)?
        };
        let ch = *chars.get(i)?;
        if is_emoji_modifier(ch) {
            continue;
        }
        return Some(ch);
    }
}

fn emoji_joiner_context(chars: &[char], index: usize) -> bool {
    let before = neighbour(chars, index, false);
    let after = neighbour(chars, index, true);
    matches!((before, after), (Some(b), Some(a)) if is_pictographic(b) && is_pictographic(a))
}

fn preceded_by_pictograph(chars: &[char], index: usize) -> bool {
    matches!(neighbour(chars, index, false), Some(b) if is_pictographic(b))
}

fn script_joiner_context(chars: &[char], index: usize) -> bool {
    let before = neighbour(chars, index, false).map(is_joining_script) == Some(true);
    let after = neighbour(chars, index, true).map(is_joining_script) == Some(true);
    before || after
}

fn adjacent_to_dash(chars: &[char], index: usize) -> bool {
    let is_dash = |c: Option<&char>| matches!(c, Some('\u{2014}') | Some('\u{2013}'));
    is_dash(index.checked_sub(1).and_then(|i| chars.get(i))) || is_dash(chars.get(index + 1))
}

fn post_process(mut text: String, profile: &Profile) -> String {
    if profile.nfkc {
        text = text.nfkc().collect();
    }
    if profile.trim_trailing_whitespace {
        let had_final_newline = text.ends_with('\n');
        let mut buf: String = text
            .split('\n')
            .map(|line| line.trim_end_matches([' ', '\t', '\r']))
            .collect::<Vec<_>>()
            .join("\n");
        if had_final_newline && !buf.ends_with('\n') {
            buf.push('\n');
        }
        text = buf;
    }
    if profile.collapse_blank_lines {
        text = collapse_blank_lines(&text);
    }
    if let Some(ending) = profile.line_ending {
        text = text.replace("\r\n", "\n");
        if ending == LineEnding::Crlf {
            text = text.replace('\n', "\r\n");
        }
    }
    text
}

fn collapse_blank_lines(text: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut blank_run = 0usize;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        kept.push(line);
    }
    kept.join("\n")
}

fn signals_for(chars: &[char]) -> Vec<Signal> {
    let total = chars.len().max(1);
    let mut nnbsp_dash = 0usize;
    let mut em_dash = 0usize;
    let mut curly = 0usize;
    let mut ellipsis = 0usize;
    let mut nbsp = 0usize;

    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            '\u{2014}' => em_dash += 1,
            '\u{2018}' | '\u{2019}' | '\u{201C}' | '\u{201D}' => curly += 1,
            '\u{2026}' => ellipsis += 1,
            '\u{00A0}' => nbsp += 1,
            '\u{202F}' if adjacent_to_dash(chars, i) => nnbsp_dash += 1,
            _ => {}
        }
    }

    let mk = |kind: SignalKind, count: usize| Signal {
        kind,
        count,
        per_1k_x100: ((count as u64 * 100_000) / total as u64) as u32,
    };

    [
        (SignalKind::NarrowSpaceEmDash, nnbsp_dash),
        (SignalKind::EmDashDensity, em_dash),
        (SignalKind::CurlyQuotes, curly),
        (SignalKind::UnicodeEllipsis, ellipsis),
        (SignalKind::NonBreakingSpaces, nbsp),
    ]
    .into_iter()
    .filter(|(_, c)| *c > 0)
    .map(|(k, c)| mk(k, c))
    .collect()
}

fn stats_for(input: &str, cleaned: &str, findings: &[Finding]) -> Stats {
    let mut by_category: BTreeMap<Category, usize> = BTreeMap::new();
    let mut removed = 0;
    let mut replaced = 0;
    let mut kept = 0;

    for f in findings {
        *by_category.entry(f.category).or_insert(0) += 1;
        match f.action {
            Action::Removed => removed += 1,
            Action::Replaced(_) => replaced += 1,
            Action::Kept => kept += 1,
        }
    }

    Stats {
        input_chars: input.chars().count(),
        input_bytes: input.len(),
        output_chars: cleaned.chars().count(),
        output_bytes: cleaned.len(),
        removed,
        replaced,
        kept,
        by_category: by_category.into_iter().collect(),
    }
}

/// Triage score. Deliberately blunt: the categories dominate, volume only
/// nudges. It is a sorting aid for humans, never a probability.
fn score_for(findings: &[Finding], payloads: &[HiddenPayload]) -> (u8, Verdict) {
    if !payloads.is_empty() {
        return (100, Verdict::HiddenContent);
    }

    let mut base = 0u32;
    let mut volume = 0u32;

    for f in findings {
        if f.action == Action::Kept && f.note.is_some() && f.note != Some(Note::PartOfPayload) {
            // A legitimate emoji or script joiner is not evidence of anything.
            continue;
        }
        let weight = match f.category {
            Category::Tag => 95,
            Category::Invisible => 65,
            Category::Bidi => 60,
            Category::Homoglyph => 55,
            Category::VariationSelector => 50,
            Category::Deprecated => 40,
            Category::Space => 18,
            Category::Typography => 4,
        };
        base = base.max(weight);
        volume += 1;
    }

    if base == 0 {
        return (0, Verdict::Clean);
    }

    let score = (base + volume.min(15)).min(100) as u8;
    let verdict = if score >= 45 {
        Verdict::Suspicious
    } else if score >= 10 {
        Verdict::Cosmetic
    } else {
        Verdict::Clean
    };
    (score, verdict)
}

/// Whether `report` should make `no-watermark check` exit non-zero.
///
/// Findings that were deliberately kept because they are legitimate (an emoji
/// joiner, a Persian ZWNJ) never fail the check.
pub fn fails_check(report: &Report, min: Severity) -> bool {
    if !report.payloads.is_empty() {
        return true;
    }
    report.findings.iter().any(|f| {
        f.severity >= min
            && !(f.action == Action::Kept
                && matches!(
                    f.note,
                    Some(Note::LegitimateEmojiJoiner) | Some(Note::LegitimateScriptJoiner)
                ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_zero_width_space() {
        let r = analyze("a\u{200B}b", &Profile::safe());
        assert_eq!(r.cleaned, "ab");
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].category, Category::Invisible);
        assert_eq!(r.findings[0].action, Action::Removed);
    }

    #[test]
    fn scan_profile_never_modifies() {
        let dirty = "a\u{200B}b\u{202F}c\u{E0041}";
        let r = analyze(dirty, &Profile::scan());
        assert_eq!(r.cleaned, dirty);
        assert!(!r.modified());
        assert!(!r.findings.is_empty());
    }

    #[test]
    fn decodes_unicode_tags_payload() {
        // "Hi" mirrored into the Tags block.
        let hidden: String = "Hi".chars().map(tag).collect();
        let text = format!("Report{hidden}.");
        let r = analyze(&text, &Profile::safe());
        assert_eq!(r.cleaned, "Report.");
        assert_eq!(r.payloads.len(), 1);
        assert_eq!(r.payloads[0].kind, PayloadKind::UnicodeTags);
        assert_eq!(r.payloads[0].text.as_deref(), Some("Hi"));
        assert_eq!(r.verdict, Verdict::HiddenContent);
        assert_eq!(r.score, 100);
    }

    #[test]
    fn decodes_variation_selector_payload() {
        let hidden: String = b"abc".iter().map(|b| vs(*b)).collect();
        let text = format!("x{hidden}y");
        let r = analyze(&text, &Profile::safe());
        assert_eq!(r.cleaned, "xy");
        assert_eq!(r.payloads.len(), 1);
        assert_eq!(r.payloads[0].text.as_deref(), Some("abc"));
    }

    #[test]
    fn decodes_zero_width_binary_payload() {
        let bits: String = b"Hi"
            .iter()
            .flat_map(|b| (0..8).rev().map(move |i| (b >> i) & 1))
            .map(|bit| if bit == 1 { '\u{200C}' } else { '\u{200B}' })
            .collect();
        let r = analyze(&format!("note{bits}"), &Profile::safe());
        assert_eq!(r.cleaned, "note");
        assert_eq!(r.payloads.len(), 1);
        assert_eq!(r.payloads[0].kind, PayloadKind::ZeroWidthBinary);
        assert_eq!(r.payloads[0].text.as_deref(), Some("Hi"));
    }

    #[test]
    fn preserves_emoji_zwj_sequence() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let r = analyze(family, &Profile::standard());
        assert_eq!(r.cleaned, family);
        assert!(r.findings.iter().all(|f| f.action == Action::Kept));
        assert_eq!(r.verdict, Verdict::Clean);
    }

    #[test]
    fn preserves_emoji_variation_selector() {
        let text = "\u{2764}\u{FE0F}";
        let r = analyze(text, &Profile::standard());
        assert_eq!(r.cleaned, text);
    }

    #[test]
    fn preserves_persian_zwnj() {
        let text = "\u{0645}\u{06CC}\u{200C}\u{0631}\u{0648}\u{0645}";
        let r = analyze(text, &Profile::standard());
        assert_eq!(r.cleaned, text);
    }

    #[test]
    fn normalizes_narrow_no_break_space() {
        let r = analyze("word\u{202F}\u{2014} word", &Profile::standard());
        assert_eq!(r.cleaned, "word \u{2014} word");
        assert!(r
            .findings
            .iter()
            .any(|f| f.note == Some(Note::NarrowSpaceAroundDash)));
    }

    #[test]
    fn aggressive_flattens_typography() {
        let r = analyze("“Hi” — it’s fine…", &Profile::aggressive());
        assert_eq!(r.cleaned, "\"Hi\" - it's fine...");
    }

    #[test]
    fn standard_leaves_typography_alone() {
        let text = "“Hi” — it’s fine…";
        let r = analyze(text, &Profile::standard());
        assert_eq!(r.cleaned, text);
    }

    #[test]
    fn folds_homoglyph_inside_latin_word() {
        // Cyrillic 'о' inside an otherwise Latin word.
        let r = analyze("passw\u{043E}rd", &Profile::standard());
        assert_eq!(r.cleaned, "password");
        assert_eq!(r.findings[0].category, Category::Homoglyph);
    }

    #[test]
    fn leaves_genuine_cyrillic_alone() {
        let text = "привет мир";
        let r = analyze(text, &Profile::aggressive());
        assert_eq!(r.cleaned, text);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn strips_bidi_override() {
        let r = analyze("safe\u{202E}elif.txt", &Profile::safe());
        assert_eq!(r.cleaned, "safeelif.txt");
        assert_eq!(r.findings[0].category, Category::Bidi);
    }

    #[test]
    fn clean_text_scores_zero() {
        let r = analyze("Just an ordinary sentence.", &Profile::standard());
        assert_eq!(r.score, 0);
        assert_eq!(r.verdict, Verdict::Clean);
        assert!(!r.modified());
    }

    #[test]
    fn bom_is_reported_at_start() {
        let r = analyze("\u{FEFF}text", &Profile::safe());
        assert_eq!(r.cleaned, "text");
        assert_eq!(r.findings[0].note, Some(Note::ByteOrderMark));
    }

    #[test]
    fn visualize_marks_invisibles() {
        assert_eq!(visualize("a\u{200B}b"), "a⟦U+200B⟧b");
        assert_eq!(visualize("a\u{00A0}b"), "a␣b");
    }

    #[test]
    fn trailing_whitespace_trimmed_in_standard() {
        let r = analyze("line   \nnext\t\n", &Profile::standard());
        assert_eq!(r.cleaned, "line\nnext\n");
    }

    #[test]
    fn positions_are_reported_per_line() {
        let r = analyze("ok\nbad\u{200B}\n", &Profile::safe());
        assert_eq!(r.findings[0].line, 2);
        assert_eq!(r.findings[0].column, 4);
    }

    #[test]
    fn idempotent_cleaning() {
        let dirty = "a\u{200B}b\u{202F}c\u{E0041}\u{2014}";
        let once = clean(dirty, &Profile::aggressive());
        let twice = clean(&once, &Profile::aggressive());
        assert_eq!(once, twice);
    }

    fn tag(c: char) -> char {
        char::from_u32(0xE0000 + c as u32).unwrap()
    }

    fn vs(b: u8) -> char {
        if b < 16 {
            char::from_u32(0xFE00 + b as u32).unwrap()
        } else {
            char::from_u32(0xE0100 + b as u32 - 16).unwrap()
        }
    }
}
