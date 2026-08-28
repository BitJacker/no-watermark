//! English and Italian strings for no-watermark.
//!
//! `nowm-core` only ever emits stable identifiers; this crate turns them into
//! prose. Both languages are compiled in, so a build never depends on locale
//! files being installed next to the binary.

use std::fmt;

use nowm_core::report::{Category, Note, PayloadKind, Severity, SignalKind, Verdict};
use nowm_core::Preset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    #[default]
    En,
    It,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::It];

    /// BCP-47-ish tag, as accepted on the command line.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::It => "it",
        }
    }

    /// Name of the language, written in that language.
    pub fn endonym(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::It => "Italiano",
        }
    }

    pub fn parse(s: &str) -> Option<Lang> {
        let s = s.trim().to_ascii_lowercase();
        let primary = s.split(['-', '_', '.']).next().unwrap_or("");
        match primary {
            "en" => Some(Lang::En),
            "it" => Some(Lang::It),
            _ => None,
        }
    }

    /// Resolve the interface language.
    ///
    /// `NOWM_LANG` wins, then the operating system locale, then English.
    pub fn detect() -> Lang {
        if let Ok(v) = std::env::var("NOWM_LANG") {
            if let Some(l) = Lang::parse(&v) {
                return l;
            }
        }
        sys_locale::get_locale()
            .and_then(|l| Lang::parse(&l))
            .unwrap_or(Lang::En)
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

macro_rules! strings {
    ($($key:literal => ($en:literal, $it:literal)),* $(,)?) => {
        /// Look up a key. Returns `None` for unknown keys.
        pub fn lookup(lang: Lang, key: &str) -> Option<&'static str> {
            match key {
                $($key => Some(match lang { Lang::En => $en, Lang::It => $it }),)*
                _ => None,
            }
        }

        /// Every key known to this crate, used by the coverage test.
        pub const KEYS: &[&str] = &[$($key),*];
    };
}

/// Translate `key`, falling back to the key itself so a missing string is
/// visible rather than silently empty.
pub fn t(lang: Lang, key: &str) -> &'static str {
    lookup(lang, key).unwrap_or_else(|| leak_key(key))
}

/// Translate and substitute `{0}`, `{1}`, … positionally.
pub fn tf(lang: Lang, key: &str, args: &[&str]) -> String {
    let mut s = t(lang, key).to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    s
}

fn leak_key(key: &str) -> &'static str {
    // Only reachable for a key that is not in the table, i.e. a bug. Leaking a
    // short string once is preferable to panicking in front of a user.
    Box::leak(key.to_string().into_boxed_str())
}

pub fn category(lang: Lang, c: Category) -> &'static str {
    t(lang, &format!("category.{}", c.key()))
}

pub fn category_desc(lang: Lang, c: Category) -> &'static str {
    t(lang, &format!("category.{}.desc", c.key()))
}

pub fn severity(lang: Lang, s: Severity) -> &'static str {
    t(lang, &format!("severity.{}", s.key()))
}

pub fn note(lang: Lang, n: Note) -> &'static str {
    t(lang, &format!("note.{}", n.key()))
}

pub fn payload_kind(lang: Lang, p: PayloadKind) -> &'static str {
    t(lang, &format!("payload.{}", p.key()))
}

pub fn verdict(lang: Lang, v: Verdict) -> &'static str {
    t(lang, &format!("verdict.{}", v.key()))
}

pub fn verdict_desc(lang: Lang, v: Verdict) -> &'static str {
    t(lang, &format!("verdict.{}.desc", v.key()))
}

pub fn signal(lang: Lang, s: SignalKind) -> &'static str {
    t(lang, &format!("signal.{}", s.key()))
}

pub fn signal_desc(lang: Lang, s: SignalKind) -> &'static str {
    t(lang, &format!("signal.{}.desc", s.key()))
}

pub fn preset(lang: Lang, p: Preset) -> &'static str {
    t(lang, &format!("preset.{}", p.key()))
}

pub fn preset_desc(lang: Lang, p: Preset) -> &'static str {
    t(lang, &format!("preset.{}.desc", p.key()))
}

strings! {
    // ---- categories --------------------------------------------------------
    "category.tag" => ("Unicode tag character", "Carattere tag Unicode"),
    "category.tag.desc" => (
        "A shadow copy of ASCII (U+E0000-U+E007F) that renders as nothing but is read normally by a language model. There is no legitimate use for it in chat text.",
        "Una copia ombra dell'ASCII (U+E0000-U+E007F) che non viene disegnata a schermo ma che un modello linguistico legge normalmente. Non ha usi legittimi in un testo di chat."
    ),
    "category.invisible" => ("Invisible character", "Carattere invisibile"),
    "category.invisible.desc" => (
        "Zero-width and format characters that take up no space on screen. Used to fingerprint text or to hide instructions.",
        "Caratteri a larghezza zero e di formattazione che non occupano spazio a schermo. Usati per marcare il testo o per nascondere istruzioni."
    ),
    "category.bidi" => ("Bidirectional control", "Controllo bidirezionale"),
    "category.bidi.desc" => (
        "Reorders how text is displayed, so what you see can differ from what is stored.",
        "Riordina la visualizzazione del testo: quello che vedi puo' differire da quello che e' memorizzato."
    ),
    "category.variation_selector" => ("Variation selector", "Selettore di variante"),
    "category.variation_selector.desc" => (
        "Invisible modifiers meant for emoji presentation. A run of them can carry one byte each.",
        "Modificatori invisibili pensati per la resa delle emoji. Una sequenza puo' trasportare un byte ciascuno."
    ),
    "category.homoglyph" => ("Confusable letter", "Lettera confondibile"),
    "category.homoglyph.desc" => (
        "A non-Latin letter impersonating a Latin one, for example Cyrillic 'o' inside an English word.",
        "Una lettera non latina che ne imita una latina, ad esempio la 'o' cirillica dentro una parola italiana."
    ),
    "category.space" => ("Whitespace look-alike", "Spazio anomalo"),
    "category.space.desc" => (
        "Whitespace that is not a plain space: non-breaking, narrow, thin, ideographic and similar.",
        "Spaziatura diversa dallo spazio normale: unificatore, stretto, sottile, ideografico e simili."
    ),
    "category.typography" => ("Typography", "Tipografia"),
    "category.typography.desc" => (
        "Curly quotes, long dashes and single-character ellipses. Cosmetic, not a watermark.",
        "Virgolette curve, trattini lunghi e puntini di sospensione in un solo carattere. Cosmetico, non un watermark."
    ),
    "category.deprecated" => ("Deprecated format character", "Carattere di formato deprecato"),
    "category.deprecated.desc" => (
        "Format characters Unicode itself has deprecated. Their presence in modern text is anomalous.",
        "Caratteri di formato che Unicode stesso ha deprecato. La loro presenza in un testo moderno e' anomala."
    ),

    // ---- severities --------------------------------------------------------
    "severity.info" => ("info", "informativo"),
    "severity.low" => ("low", "basso"),
    "severity.medium" => ("medium", "medio"),
    "severity.high" => ("high", "alto"),
    "severity.critical" => ("critical", "critico"),

    // ---- notes -------------------------------------------------------------
    "note.legitimate_emoji_joiner" => (
        "legitimate: holds an emoji sequence together",
        "legittimo: tiene unita una sequenza emoji"
    ),
    "note.legitimate_script_joiner" => (
        "legitimate: orthographic joiner in this script",
        "legittimo: giunzione ortografica in questa scrittura"
    ),
    "note.byte_order_mark" => ("byte order mark at start of file", "byte order mark a inizio file"),
    "note.part_of_payload" => ("carries hidden content", "trasporta contenuto nascosto"),
    "note.narrow_space_around_dash" => (
        "narrow no-break space next to a dash",
        "spazio stretto unificatore accanto a un trattino"
    ),

    // ---- payload kinds -----------------------------------------------------
    "payload.unicode_tags" => ("Unicode tags", "Tag Unicode"),
    "payload.variation_selector" => ("Variation selectors", "Selettori di variante"),
    "payload.zero_width_binary" => ("Zero-width binary", "Binario a larghezza zero"),

    // ---- verdicts ----------------------------------------------------------
    "verdict.clean" => ("Clean", "Pulito"),
    "verdict.clean.desc" => (
        "No invisible or deceptive characters found.",
        "Nessun carattere invisibile o ingannevole trovato."
    ),
    "verdict.cosmetic" => ("Cosmetic only", "Solo cosmetico"),
    "verdict.cosmetic.desc" => (
        "Only typographic or whitespace oddities. Nothing is hidden.",
        "Solo anomalie tipografiche o di spaziatura. Non c'e' nulla di nascosto."
    ),
    "verdict.suspicious" => ("Suspicious", "Sospetto"),
    "verdict.suspicious.desc" => (
        "Invisible or deceptive characters are present.",
        "Sono presenti caratteri invisibili o ingannevoli."
    ),
    "verdict.hidden_content" => ("Hidden content", "Contenuto nascosto"),
    "verdict.hidden_content.desc" => (
        "Readable content was recovered from inside the text.",
        "E' stato recuperato del contenuto leggibile nascosto nel testo."
    ),

    // ---- stylometric signals -----------------------------------------------
    "signal.narrow_space_em_dash" => (
        "Narrow no-break space next to em dash",
        "Spazio stretto unificatore accanto a lineetta"
    ),
    "signal.narrow_space_em_dash.desc" => (
        "The pattern reported in 2025 for some ChatGPT models. OpenAI called it a tokenisation artefact, not a watermark.",
        "Il pattern segnalato nel 2025 per alcuni modelli ChatGPT. OpenAI lo ha definito un artefatto di tokenizzazione, non un watermark."
    ),
    "signal.em_dash_density" => ("Em dash usage", "Uso della lineetta"),
    "signal.em_dash_density.desc" => (
        "Heavy em dash use is a style habit of several assistants. On its own it proves nothing.",
        "L'uso massiccio della lineetta e' un'abitudine stilistica di diversi assistenti. Da solo non prova nulla."
    ),
    "signal.curly_quotes" => ("Typographic quotes", "Virgolette tipografiche"),
    "signal.curly_quotes.desc" => (
        "Curly quotation marks instead of straight ASCII ones.",
        "Virgolette curve invece di quelle dritte ASCII."
    ),
    "signal.unicode_ellipsis" => ("Single-character ellipsis", "Puntini in un solo carattere"),
    "signal.unicode_ellipsis.desc" => (
        "U+2026 instead of three full stops.",
        "U+2026 invece di tre punti."
    ),
    "signal.non_breaking_spaces" => ("Non-breaking spaces", "Spazi unificatori"),
    "signal.non_breaking_spaces.desc" => (
        "U+00A0 scattered through ordinary prose.",
        "U+00A0 sparsi in un testo normale."
    ),

    // ---- presets -----------------------------------------------------------
    "preset.scan" => ("Scan", "Analisi"),
    "preset.scan.desc" => (
        "Report only. The output is byte-for-byte the input.",
        "Solo analisi. L'output e' identico all'input, byte per byte."
    ),
    "preset.safe" => ("Safe", "Sicuro"),
    "preset.safe.desc" => (
        "Remove invisible and deceptive characters. Every visible glyph is preserved.",
        "Rimuove i caratteri invisibili e ingannevoli. Ogni glifo visibile viene preservato."
    ),
    "preset.standard" => ("Standard", "Standard"),
    "preset.standard.desc" => (
        "Safe, plus normalised whitespace and confusable letters folded to ASCII.",
        "Come Sicuro, piu' spaziatura normalizzata e lettere confondibili riportate ad ASCII."
    ),
    "preset.aggressive" => ("Aggressive", "Aggressivo"),
    "preset.aggressive.desc" => (
        "Standard, plus NFKC normalisation and ASCII-only punctuation. Changes how the text looks.",
        "Come Standard, piu' normalizzazione NFKC e punteggiatura solo ASCII. Cambia l'aspetto del testo."
    ),

    // ---- actions -----------------------------------------------------------
    "action.removed" => ("removed", "rimosso"),
    "action.replaced" => ("replaced", "sostituito"),
    "action.kept" => ("kept", "mantenuto"),

    // ---- shared report vocabulary ------------------------------------------
    "report.title" => ("no-watermark report", "Rapporto no-watermark"),
    "report.verdict" => ("Verdict", "Verdetto"),
    "report.score" => ("Score", "Punteggio"),
    "report.findings" => ("Findings", "Rilevamenti"),
    "report.no_findings" => ("Nothing suspicious found.", "Nessuna anomalia trovata."),
    "report.hidden_payloads" => ("Hidden content recovered", "Contenuto nascosto recuperato"),
    "report.signals" => ("Style signals", "Segnali stilistici"),
    "report.summary" => ("Summary", "Riepilogo"),
    "report.removed_n" => ("{0} removed", "{0} rimossi"),
    "report.replaced_n" => ("{0} replaced", "{0} sostituiti"),
    "report.kept_n" => ("{0} kept", "{0} mantenuti"),
    "report.chars_in_out" => ("{0} characters in, {1} out", "{0} caratteri in ingresso, {1} in uscita"),
    "report.position" => ("line {0}, column {1}", "riga {0}, colonna {1}"),
    "report.occurrences.one" => ("{0} occurrence", "{0} occorrenza"),
    "report.occurrences.many" => ("{0} occurrences", "{0} occorrenze"),
    "report.decoded_as_text" => ("decoded text", "testo decodificato"),
    "report.decoded_as_hex" => ("raw bytes (hex)", "byte grezzi (hex)"),
    "report.payload_warning" => (
        "This text carried content that is invisible to you but readable by an AI model. Treat it as untrusted.",
        "Questo testo trasportava contenuto invisibile per te ma leggibile da un modello di IA. Trattalo come non attendibile."
    ),

    // ---- limits ------------------------------------------------------------
    "limits.title" => ("What this tool cannot do", "Cosa questo strumento non puo' fare"),
    "limits.body" => (
        "no-watermark works on characters. It cannot remove a statistical watermark such as SynthID-Text, which lives in the model's token sampling and leaves no special character behind. No character-level tool can.",
        "no-watermark lavora sui caratteri. Non puo' rimuovere un watermark statistico come SynthID-Text, che risiede nel campionamento dei token del modello e non lascia alcun carattere speciale. Nessuno strumento a livello di carattere puo' farlo."
    ),

    // ---- CLI ---------------------------------------------------------------
    "cli.clean_ok" => ("Cleaned: {0}", "Pulito: {0}"),
    "cli.unchanged" => ("Unchanged: {0}", "Invariato: {0}"),
    "cli.check_failed" => (
        "Check failed: suspicious characters found.",
        "Controllo fallito: trovati caratteri sospetti."
    ),
    "cli.check_passed" => ("Check passed.", "Controllo superato."),
    "cli.reading_stdin" => ("Reading from standard input.", "Lettura dallo standard input."),
    "cli.no_input" => (
        "No input given. Pass a file, or pipe text on standard input.",
        "Nessun input fornito. Passa un file o invia il testo sullo standard input."
    ),
    "cli.files_scanned" => ("{0} files scanned", "{0} file analizzati"),
    "cli.files_modified" => ("{0} files modified", "{0} file modificati"),
    "cli.wrote" => ("Wrote {0}", "Scritto {0}"),
    "cli.dry_run" => ("Dry run: nothing was written.", "Prova a vuoto: non e' stato scritto nulla."),
    "cli.backup_written" => ("Backup written to {0}", "Backup scritto in {0}"),

    // ---- GUI ---------------------------------------------------------------
    "ui.window_title" => ("no-watermark", "no-watermark"),
    "ui.tagline" => (
        "Strip invisible watermarks from AI chat output",
        "Rimuovi i watermark invisibili dall'output delle chat IA"
    ),
    "ui.input" => ("Input", "Input"),
    "ui.output" => ("Cleaned output", "Output pulito"),
    "ui.paste" => ("Paste", "Incolla"),
    "ui.copy" => ("Copy", "Copia"),
    "ui.clear" => ("Clear", "Svuota"),
    "ui.open_file" => ("Open file", "Apri file"),
    "ui.save_file" => ("Save as", "Salva come"),
    "ui.clean_now" => ("Clean", "Pulisci"),
    "ui.profile" => ("Profile", "Profilo"),
    "ui.language" => ("Language", "Lingua"),
    "ui.theme" => ("Theme", "Tema"),
    "ui.theme_dark" => ("Dark", "Scuro"),
    "ui.theme_light" => ("Light", "Chiaro"),
    "ui.options" => ("Options", "Opzioni"),
    "ui.details" => ("Details", "Dettagli"),
    "ui.show_invisibles" => ("Reveal invisible characters", "Mostra i caratteri invisibili"),
    "ui.auto_clean" => ("Clean as I type", "Pulisci mentre scrivo"),
    "ui.watch_clipboard" => ("Watch the clipboard", "Sorveglia gli appunti"),
    "ui.watch_clipboard_hint" => (
        "Clean anything you copy, automatically.",
        "Pulisce automaticamente tutto cio' che copi."
    ),
    "ui.about" => ("About", "Informazioni"),
    "ui.about_body" => (
        "no-watermark is free software released under the MIT licence.",
        "no-watermark e' software libero rilasciato con licenza MIT."
    ),
    "ui.made_by" => ("Made by Giacomo Giordano", "Realizzato da Giacomo Giordano"),
    "ui.copied" => ("Copied.", "Copiato."),
    "ui.clipboard_error" => ("The clipboard is not available.", "Gli appunti non sono disponibili."),
    "ui.clipboard_cleaned" => (
        "Clipboard cleaned: {0} characters handled.",
        "Appunti puliti: {0} caratteri trattati."
    ),
    "ui.pasted" => ("Pasted.", "Incollato."),
    "ui.saved" => ("Saved.", "Salvato."),
    "ui.nothing_to_copy" => ("There is nothing to copy.", "Non c'e' nulla da copiare."),
    "ui.opt_invisible" => ("Remove invisible characters", "Rimuovi i caratteri invisibili"),
    "ui.opt_bidi" => ("Remove bidirectional controls", "Rimuovi i controlli bidirezionali"),
    "ui.opt_tags" => ("Remove Unicode tag characters", "Rimuovi i caratteri tag Unicode"),
    "ui.opt_vs" => ("Remove variation selectors", "Rimuovi i selettori di variante"),
    "ui.opt_spaces" => ("Normalise whitespace", "Normalizza la spaziatura"),
    "ui.opt_homoglyphs" => ("Fold confusable letters", "Riporta le lettere confondibili"),
    "ui.opt_typography" => ("ASCII punctuation", "Punteggiatura ASCII"),
    "ui.opt_nfkc" => ("Unicode NFKC normalisation", "Normalizzazione Unicode NFKC"),
    "ui.opt_emoji" => ("Keep emoji joiners", "Mantieni le giunzioni emoji"),
    "ui.opt_script" => ("Keep script joiners", "Mantieni le giunzioni di scrittura"),
    "ui.opt_trim" => ("Trim trailing whitespace", "Elimina gli spazi a fine riga"),
    "ui.opt_blank" => ("Collapse blank lines", "Riduci le righe vuote"),
    "ui.placeholder_input" => (
        "Paste the text from ChatGPT, Claude, Gemini or anywhere else.",
        "Incolla qui il testo da ChatGPT, Claude, Gemini o da qualunque altra fonte."
    ),
    "ui.stat_removed" => ("Removed", "Rimossi"),
    "ui.stat_replaced" => ("Replaced", "Sostituiti"),
    "ui.stat_kept" => ("Kept", "Mantenuti"),
    "ui.stat_chars" => ("Characters", "Caratteri"),
    "ui.col_position" => ("Position", "Posizione"),
    "ui.col_char" => ("Character", "Carattere"),
    "ui.col_name" => ("Name", "Nome"),
    "ui.col_category" => ("Category", "Categoria"),
    "ui.col_action" => ("Action", "Azione"),
    "ui.col_note" => ("Note", "Nota"),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_resolves_in_both_languages() {
        for key in KEYS {
            for lang in Lang::ALL {
                let v = lookup(lang, key).unwrap_or_else(|| panic!("missing {key} for {lang}"));
                assert!(!v.is_empty(), "empty value for {key} in {lang}");
            }
        }
    }

    #[test]
    fn every_core_identifier_has_a_string() {
        for c in Category::ALL {
            for lang in Lang::ALL {
                assert!(lookup(lang, &format!("category.{}", c.key())).is_some());
                assert!(lookup(lang, &format!("category.{}.desc", c.key())).is_some());
            }
        }
        for p in Preset::ALL {
            for lang in Lang::ALL {
                assert!(lookup(lang, &format!("preset.{}", p.key())).is_some());
                assert!(lookup(lang, &format!("preset.{}.desc", p.key())).is_some());
            }
        }
    }

    #[test]
    fn placeholders_are_substituted() {
        assert_eq!(tf(Lang::En, "report.removed_n", &["3"]), "3 removed");
        assert_eq!(tf(Lang::It, "report.removed_n", &["3"]), "3 rimossi");
    }

    #[test]
    fn language_tags_parse() {
        assert_eq!(Lang::parse("it-IT"), Some(Lang::It));
        assert_eq!(Lang::parse("en_US.UTF-8"), Some(Lang::En));
        assert_eq!(Lang::parse("de"), None);
    }

    #[test]
    fn english_and_italian_differ_where_it_matters() {
        // A guard against copy-pasting the English string into the Italian slot.
        let identical = KEYS
            .iter()
            .filter(|k| lookup(Lang::En, k) == lookup(Lang::It, k))
            .count();
        assert!(
            identical * 4 < KEYS.len(),
            "too many Italian strings are identical to the English ones"
        );
    }
}
