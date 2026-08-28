//! Human-readable rendering of a [`Report`].
//!
//! ANSI colour is emitted by hand rather than through a dependency: the set of
//! escapes needed here is five constants, and keeping the CLI free of extra
//! crates means it builds on a bare server with no system libraries at all.

use nowm_core::{Action, Finding, Report, Severity, Verdict};
use nowm_i18n::{self as i18n, Lang};

pub struct Style {
    pub color: bool,
}

impl Style {
    const RESET: &'static str = "\x1b[0m";
    const BOLD: &'static str = "\x1b[1m";
    const DIM: &'static str = "\x1b[2m";
    const RED: &'static str = "\x1b[31m";
    const YELLOW: &'static str = "\x1b[33m";
    const GREEN: &'static str = "\x1b[32m";
    const BLUE: &'static str = "\x1b[34m";
    const MAGENTA: &'static str = "\x1b[35m";

    fn wrap(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("{code}{s}{}", Self::RESET)
        } else {
            s.to_string()
        }
    }

    pub fn bold(&self, s: &str) -> String {
        self.wrap(Self::BOLD, s)
    }
    pub fn dim(&self, s: &str) -> String {
        self.wrap(Self::DIM, s)
    }
    pub fn severity(&self, sev: Severity, s: &str) -> String {
        let code = match sev {
            Severity::Critical => Self::MAGENTA,
            Severity::High => Self::RED,
            Severity::Medium => Self::YELLOW,
            Severity::Low => Self::BLUE,
            Severity::Info => Self::DIM,
        };
        self.wrap(code, s)
    }
    pub fn verdict(&self, v: Verdict, s: &str) -> String {
        let code = match v {
            Verdict::HiddenContent => Self::MAGENTA,
            Verdict::Suspicious => Self::RED,
            Verdict::Cosmetic => Self::YELLOW,
            Verdict::Clean => Self::GREEN,
        };
        self.wrap(&format!("{}{code}", Self::BOLD), s)
    }
}

/// Full report: verdict, findings, recovered payloads, style signals.
pub fn report(out: &mut String, r: &Report, lang: Lang, st: &Style, label: Option<&str>) {
    if let Some(label) = label {
        out.push_str(&st.bold(label));
        out.push('\n');
    }

    out.push_str(&format!(
        "{}: {}  {}: {}/100\n",
        i18n::t(lang, "report.verdict"),
        st.verdict(r.verdict, i18n::verdict(lang, r.verdict)),
        i18n::t(lang, "report.score"),
        r.score
    ));
    out.push_str(&st.dim(i18n::verdict_desc(lang, r.verdict)));
    out.push('\n');

    if !r.payloads.is_empty() {
        out.push('\n');
        out.push_str(&st.bold(i18n::t(lang, "report.hidden_payloads")));
        out.push('\n');
        out.push_str(&st.severity(Severity::Critical, i18n::t(lang, "report.payload_warning")));
        out.push('\n');
        for p in &r.payloads {
            out.push_str(&format!(
                "  - {} ({} chars @ {})\n",
                i18n::payload_kind(lang, p.kind),
                p.len_chars,
                p.start_char
            ));
            match &p.text {
                Some(text) => out.push_str(&format!(
                    "    {}: {}\n",
                    i18n::t(lang, "report.decoded_as_text"),
                    st.bold(&escape_one_line(text))
                )),
                None => out.push_str(&format!(
                    "    {}: {}\n",
                    i18n::t(lang, "report.decoded_as_hex"),
                    p.hex
                )),
            }
        }
    }

    out.push('\n');
    out.push_str(&st.bold(i18n::t(lang, "report.findings")));
    out.push('\n');
    if r.findings.is_empty() {
        out.push_str(&format!("  {}\n", i18n::t(lang, "report.no_findings")));
    } else {
        // A smuggled payload is dozens of identical tag characters in a row.
        // Printing one line each buries everything else, so runs are collapsed.
        for (f, run) in group_runs(&r.findings) {
            let position = i18n::tf(
                lang,
                "report.position",
                &[&f.line.to_string(), &f.column.to_string()],
            );
            let position = if run > 1 {
                format!("{position} x{run}")
            } else {
                position
            };
            let action = match &f.action {
                Action::Removed => i18n::t(lang, "action.removed").to_string(),
                Action::Replaced(v) => {
                    format!("{} -> {:?}", i18n::t(lang, "action.replaced"), v.as_str())
                }
                Action::Kept => i18n::t(lang, "action.kept").to_string(),
            };
            // Pad before colouring: escape sequences would otherwise count
            // towards the field width and break every column.
            let severity = format!("{:<12}", i18n::severity(lang, f.severity));
            let identity = if run > 1 {
                // A run spans several codepoints (32 different tag characters
                // spell 32 different letters), so only the name is shared.
                format!("{:<38}", f.name)
            } else {
                format!("{:<38}", format!("{} {}", f.display, f.name))
            };
            out.push_str(&format!(
                "  {} {:<22} {} {:<12} {}\n",
                st.severity(f.severity, &severity),
                position,
                identity,
                i18n::category(lang, f.category),
                st.dim(&action),
            ));
            if let Some(n) = f.note {
                out.push_str(&format!("            {}\n", st.dim(i18n::note(lang, n))));
            }
        }
    }

    if !r.signals.is_empty() {
        out.push('\n');
        out.push_str(&st.bold(i18n::t(lang, "report.signals")));
        out.push('\n');
        for s in &r.signals {
            out.push_str(&format!(
                "  - {}: {}\n    {}\n",
                i18n::signal(lang, s.kind),
                occurrences(lang, s.count),
                st.dim(i18n::signal_desc(lang, s.kind)),
            ));
        }
    }

    out.push('\n');
    out.push_str(&st.bold(i18n::t(lang, "report.summary")));
    out.push('\n');
    out.push_str(&format!(
        "  {}, {}, {}\n  {}\n",
        i18n::tf(lang, "report.removed_n", &[&r.stats.removed.to_string()]),
        i18n::tf(lang, "report.replaced_n", &[&r.stats.replaced.to_string()]),
        i18n::tf(lang, "report.kept_n", &[&r.stats.kept.to_string()]),
        i18n::tf(
            lang,
            "report.chars_in_out",
            &[
                &r.stats.input_chars.to_string(),
                &r.stats.output_chars.to_string()
            ]
        ),
    ));
}

/// Compact one-line-per-file rendering, used when several files are processed.
pub fn short_line(r: &Report, lang: Lang, st: &Style, label: &str) -> String {
    let verdict = format!("{:<16}", i18n::verdict(lang, r.verdict));
    let counts = format!("{}/{}/{}", r.stats.removed, r.stats.replaced, r.stats.kept);
    format!(
        "{} {:<12} {}\n",
        st.verdict(r.verdict, &verdict),
        counts,
        label
    )
}

/// Pick the singular or plural form. Both languages need it and both agree
/// that one is special, which is all the pluralisation this tool requires.
fn occurrences(lang: Lang, n: usize) -> String {
    let key = if n == 1 {
        "report.occurrences.one"
    } else {
        "report.occurrences.many"
    };
    i18n::tf(lang, key, &[&n.to_string()])
}

fn escape_one_line(s: &str) -> String {
    s.replace('\n', "\\n").replace('\r', "\\r")
}

/// Collapse consecutive identical findings, returning the first of each run
/// together with the run length.
fn group_runs(findings: &[Finding]) -> Vec<(&Finding, usize)> {
    let mut out: Vec<(&Finding, usize)> = Vec::new();
    for f in findings {
        match out.last_mut() {
            Some((first, count))
                if first.name == f.name
                    && first.category == f.category
                    && first.action == f.action
                    && first.note == f.note =>
            {
                *count += 1;
            }
            _ => out.push((f, 1)),
        }
    }
    out
}
