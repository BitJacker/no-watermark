//! Recovery of content hidden inside otherwise innocent text.
//!
//! Detecting an invisible character is useful. Showing the user *what* was
//! written with it is what turns the tool from a cleaner into a forensic
//! instrument, so every carrier scheme here is decoded rather than merely
//! counted.

use std::collections::BTreeSet;

use crate::chars::{tag_char_ascii, variation_selector_byte};
use crate::report::{HiddenPayload, PayloadKind};

/// Zero-width characters usable as a binary carrier.
const ZW_CARRIERS: [char; 5] = [
    '\u{200B}', // ZERO WIDTH SPACE
    '\u{200C}', // ZERO WIDTH NON-JOINER
    '\u{200D}', // ZERO WIDTH JOINER
    '\u{2060}', // WORD JOINER
    '\u{FEFF}', // ZERO WIDTH NO-BREAK SPACE
];

/// Shortest variation-selector run treated as a payload rather than as emoji
/// presentation. Three carriers is three bytes: below that the false-positive
/// rate on ordinary emoji is not worth it.
const MIN_VS_RUN: usize = 3;

/// Shortest zero-width run treated as a binary payload (one byte).
const MIN_ZW_RUN: usize = 8;

pub struct PayloadScan {
    pub payloads: Vec<HiddenPayload>,
    /// Character indices that belong to a decoded payload.
    pub carrier_indices: BTreeSet<usize>,
}

pub fn scan(chars: &[char]) -> PayloadScan {
    let mut payloads = Vec::new();
    let mut carrier_indices = BTreeSet::new();

    scan_tags(chars, &mut payloads, &mut carrier_indices);
    scan_variation_selectors(chars, &mut payloads, &mut carrier_indices);
    scan_zero_width_binary(chars, &mut payloads, &mut carrier_indices);

    payloads.sort_by_key(|p| p.start_char);
    PayloadScan {
        payloads,
        carrier_indices,
    }
}

fn is_tag(ch: char) -> bool {
    (0xE0000..=0xE007F).contains(&(ch as u32))
}

fn scan_tags(chars: &[char], out: &mut Vec<HiddenPayload>, carriers: &mut BTreeSet<usize>) {
    let mut i = 0;
    while i < chars.len() {
        if !is_tag(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        let mut decoded = String::new();
        while i < chars.len() && is_tag(chars[i]) {
            if let Some(c) = tag_char_ascii(chars[i]) {
                decoded.push(c);
            }
            i += 1;
        }
        let len = i - start;
        if decoded.is_empty() {
            // Framing characters only; still a carrier, but nothing to show.
            continue;
        }
        for idx in start..i {
            carriers.insert(idx);
        }
        out.push(HiddenPayload {
            kind: PayloadKind::UnicodeTags,
            hex: to_hex(decoded.as_bytes()),
            text: Some(decoded),
            start_char: start,
            len_chars: len,
        });
    }
}

fn scan_variation_selectors(
    chars: &[char],
    out: &mut Vec<HiddenPayload>,
    carriers: &mut BTreeSet<usize>,
) {
    let mut i = 0;
    while i < chars.len() {
        if variation_selector_byte(chars[i]).is_none() {
            i += 1;
            continue;
        }
        let start = i;
        let mut bytes = Vec::new();
        while i < chars.len() {
            match variation_selector_byte(chars[i]) {
                Some(b) => {
                    bytes.push(b);
                    i += 1;
                }
                None => break,
            }
        }
        if bytes.len() < MIN_VS_RUN {
            continue;
        }
        for idx in start..i {
            carriers.insert(idx);
        }
        out.push(HiddenPayload {
            kind: PayloadKind::VariationSelector,
            text: printable_utf8(&bytes),
            hex: to_hex(&bytes),
            start_char: start,
            len_chars: i - start,
        });
    }
}

fn scan_zero_width_binary(
    chars: &[char],
    out: &mut Vec<HiddenPayload>,
    carriers: &mut BTreeSet<usize>,
) {
    let mut i = 0;
    while i < chars.len() {
        if !ZW_CARRIERS.contains(&chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && ZW_CARRIERS.contains(&chars[i]) {
            i += 1;
        }
        let run = &chars[start..i];
        if run.len() < MIN_ZW_RUN || run.len() % 8 != 0 {
            continue;
        }

        // A binary channel needs exactly two distinct symbols.
        let mut distinct: Vec<char> = run.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() != 2 {
            continue;
        }

        let (zero, one) = (distinct[0], distinct[1]);
        let mut best: Option<(Option<String>, Vec<u8>)> = None;
        for (z, o) in [(zero, one), (one, zero)] {
            let bytes = bits_to_bytes(run, z, o);
            let text = printable_utf8(&bytes);
            // Prefer the mapping that yields readable text over the one that
            // yields raw bytes; the first readable result wins.
            let better = matches!((&best, &text), (None, _) | (Some((None, _)), Some(_)));
            if better {
                best = Some((text, bytes));
            }
            if let Some((Some(_), _)) = &best {
                break;
            }
        }

        let Some((text, bytes)) = best else { continue };
        // Reject runs that decode to control-character soup: almost certainly
        // formatting noise rather than a message.
        if text.is_none() && bytes.iter().all(|b| *b == 0 || *b == 0xFF) {
            continue;
        }
        for idx in start..i {
            carriers.insert(idx);
        }
        out.push(HiddenPayload {
            kind: PayloadKind::ZeroWidthBinary,
            text,
            hex: to_hex(&bytes),
            start_char: start,
            len_chars: run.len(),
        });
    }
}

fn bits_to_bytes(run: &[char], zero: char, one: char) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(run.len() / 8);
    for chunk in run.chunks(8) {
        let mut b = 0u8;
        for &ch in chunk {
            b <<= 1;
            if ch == one {
                b |= 1;
            } else if ch != zero {
                // Should not happen: the caller guarantees two symbols.
                return bytes;
            }
        }
        bytes.push(b);
    }
    bytes
}

/// Decode as UTF-8, but only accept the result if it looks like a message a
/// human would recognise rather than random bytes that happen to be valid.
fn printable_utf8(bytes: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(bytes).ok()?;
    if s.is_empty() {
        return None;
    }
    let printable = s
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .count();
    if printable * 10 >= s.chars().count() * 8 {
        Some(s.to_string())
    } else {
        None
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0x0F) as u32, 16).unwrap());
    }
    s
}
