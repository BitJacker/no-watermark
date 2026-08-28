//! End-to-end tests that run the real binary.

use std::io::Write;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_no-watermark");

fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        // Pin the language so assertions do not depend on the developer's
        // system locale.
        .env("NOWM_LANG", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn no-watermark");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

/// "hidden" mirrored into the Unicode Tags block.
fn tagged(s: &str) -> String {
    s.chars()
        .map(|c| char::from_u32(0xE0000 + c as u32).unwrap())
        .collect()
}

#[test]
fn clean_is_the_default_subcommand() {
    let out = run(&[], "a\u{200B}b");
    assert!(out.status.success());
    assert_eq!(stdout(&out), "ab");
}

#[test]
fn clean_leaves_ordinary_text_untouched() {
    let text = "Nothing to see here.\nSecond line.\n";
    let out = run(&["clean"], text);
    assert_eq!(stdout(&out), text);
}

#[test]
fn safe_profile_keeps_typography() {
    let out = run(&["clean", "--profile", "safe"], "a\u{200B}b \u{2014} c");
    assert_eq!(stdout(&out), "ab \u{2014} c");
}

#[test]
fn aggressive_profile_flattens_typography() {
    let out = run(
        &["clean", "--profile", "aggressive"],
        "a \u{2014} \u{201C}b\u{201D}",
    );
    assert_eq!(stdout(&out), "a - \"b\"");
}

#[test]
fn keep_overrides_the_preset() {
    let out = run(&["clean", "--keep", "space"], "a\u{00A0}b");
    assert_eq!(stdout(&out), "a\u{00A0}b");
}

#[test]
fn remove_adds_to_the_preset() {
    let out = run(
        &["clean", "--profile", "safe", "--remove", "space"],
        "a\u{00A0}b",
    );
    assert_eq!(stdout(&out), "a b");
}

#[test]
fn check_fails_on_hidden_content() {
    let out = run(&["check"], &format!("visible{}", tagged("secret")));
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout(&out).contains("Check failed"));
}

#[test]
fn check_passes_on_clean_text() {
    let out = run(&["check"], "perfectly ordinary text");
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("Check passed"));
}

#[test]
fn check_severity_threshold_is_honoured() {
    // A non-breaking space is Low, so a Medium threshold must let it pass.
    let out = run(&["check", "--min-severity", "medium"], "a\u{00A0}b");
    assert_eq!(out.status.code(), Some(0));

    let out = run(&["check", "--min-severity", "low"], "a\u{00A0}b");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn decode_recovers_the_payload() {
    let out = run(&["decode"], &format!("hello{}", tagged("attack")));
    assert!(stdout(&out).contains("attack"));
}

#[test]
fn scan_reports_without_changing_anything() {
    let out = run(&["scan"], "a\u{200B}b");
    let text = stdout(&out);
    assert!(text.contains("ZERO WIDTH SPACE"));
    assert!(text.contains("0 removed"));
}

#[test]
fn reveal_makes_invisibles_visible() {
    let out = run(&["reveal"], "a\u{200B}b");
    assert_eq!(stdout(&out), "a\u{27E6}U+200B\u{27E7}b");
}

#[test]
fn json_output_is_valid_json() {
    let out = run(&["scan", "--json"], "a\u{200B}b");
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(parsed[0]["report"]["findings"][0]["codepoint"], 0x200B);
}

#[test]
fn italian_output_is_available() {
    let out = run(&["scan", "--lang", "it"], "a\u{200B}b");
    assert!(stdout(&out).contains("Verdetto"));
}

#[test]
fn language_flag_beats_the_environment() {
    let out = run(&["scan", "--lang", "en"], "a\u{200B}b");
    assert!(stdout(&out).contains("Verdict"));
}
