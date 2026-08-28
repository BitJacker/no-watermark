//! `no-watermark` — command line interface.
//!
//! Exit codes: `0` success, `1` a `check` found something, `2` an error.

mod render;

use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};

use nowm_core::{analyze, visualize, Category, Preset, Profile, Report, Severity};
use nowm_i18n::{self as i18n, Lang};

use crate::render::Style;

const AFTER_HELP_EN: &str = "\
Examples:
  no-watermark < answer.txt              clean text from standard input
  no-watermark clean -i notes/*.md       clean files in place
  no-watermark scan report.md            explain what is in a file
  no-watermark check --min-severity high notes.md
  no-watermark decode --json message.txt recover hidden content

no-watermark removes character-level fingerprints. It cannot remove a
statistical watermark such as SynthID-Text, which is embedded in a model's
token sampling and leaves no special character behind.";

#[derive(Parser, Debug)]
#[command(
    name = "no-watermark",
    version,
    about = "Detect and strip invisible Unicode watermarks from AI chat output.",
    after_long_help = AFTER_HELP_EN,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    global: GlobalArgs,
}

#[derive(Args, Debug, Clone)]
struct GlobalArgs {
    /// Interface language. Defaults to the system locale, then English.
    #[arg(long, global = true, value_name = "en|it")]
    lang: Option<String>,

    /// Cleaning profile.
    #[arg(long, short = 'p', global = true, value_enum, default_value_t = PresetArg::Standard)]
    profile: PresetArg,

    /// Categories the profile must not touch (comma separated).
    #[arg(long, global = true, value_delimiter = ',', value_name = "CATEGORY")]
    keep: Vec<CategoryArg>,

    /// Extra categories to act on (comma separated).
    #[arg(long, global = true, value_delimiter = ',', value_name = "CATEGORY")]
    remove: Vec<CategoryArg>,

    /// Also strip joiners that hold emoji sequences together.
    #[arg(long, global = true)]
    no_preserve_emoji: bool,

    /// Also strip joiners that carry meaning in Arabic or Indic scripts.
    #[arg(long, global = true)]
    no_preserve_script: bool,

    /// Rewrite line endings.
    #[arg(long, global = true, value_enum)]
    line_endings: Option<LineEndingArg>,

    /// Emit machine-readable JSON instead of prose.
    #[arg(long, global = true)]
    json: bool,

    /// When to colourise output.
    #[arg(long, global = true, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// Suppress progress and summary lines.
    #[arg(long, short = 'q', global = true)]
    quiet: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Clean text and write the result out (the default).
    Clean(CleanArgs),
    /// Report what is in the text without changing anything.
    Scan(InputArgs),
    /// Exit non-zero when suspicious characters are present.
    Check(CheckArgs),
    /// Print the text with every invisible character made visible.
    Reveal(InputArgs),
    /// Print only the content hidden inside the text.
    Decode(InputArgs),
}

#[derive(Args, Debug, Default)]
struct InputArgs {
    /// Files to read. Reads standard input when omitted.
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,
}

#[derive(Args, Debug, Default)]
struct CleanArgs {
    #[command(flatten)]
    input: InputArgs,

    /// Rewrite each file in place.
    #[arg(long, short = 'i')]
    in_place: bool,

    /// Write to this file instead of standard output.
    #[arg(long, short = 'o', value_name = "FILE", conflicts_with = "in_place")]
    output: Option<PathBuf>,

    /// With --in-place, keep the original as FILE.bak.
    #[arg(long, requires = "in_place")]
    backup: bool,

    /// Show what would change without writing anything.
    #[arg(long, short = 'n')]
    dry_run: bool,
}

#[derive(Args, Debug, Default)]
struct CheckArgs {
    #[command(flatten)]
    input: InputArgs,

    /// Lowest severity that makes the check fail.
    #[arg(long, value_enum, default_value_t = SeverityArg::Medium)]
    min_severity: SeverityArg,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum PresetArg {
    Scan,
    Safe,
    Standard,
    Aggressive,
}

impl From<PresetArg> for Preset {
    fn from(p: PresetArg) -> Self {
        match p {
            PresetArg::Scan => Preset::Scan,
            PresetArg::Safe => Preset::Safe,
            PresetArg::Standard => Preset::Standard,
            PresetArg::Aggressive => Preset::Aggressive,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum CategoryArg {
    Tag,
    Invisible,
    Bidi,
    #[value(alias = "vs")]
    VariationSelector,
    Homoglyph,
    Space,
    Typography,
    Deprecated,
}

impl From<CategoryArg> for Category {
    fn from(c: CategoryArg) -> Self {
        match c {
            CategoryArg::Tag => Category::Tag,
            CategoryArg::Invisible => Category::Invisible,
            CategoryArg::Bidi => Category::Bidi,
            CategoryArg::VariationSelector => Category::VariationSelector,
            CategoryArg::Homoglyph => Category::Homoglyph,
            CategoryArg::Space => Category::Space,
            CategoryArg::Typography => Category::Typography,
            CategoryArg::Deprecated => Category::Deprecated,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, ValueEnum)]
enum SeverityArg {
    Info,
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl From<SeverityArg> for Severity {
    fn from(s: SeverityArg) -> Self {
        match s {
            SeverityArg::Info => Severity::Info,
            SeverityArg::Low => Severity::Low,
            SeverityArg::Medium => Severity::Medium,
            SeverityArg::High => Severity::High,
            SeverityArg::Critical => Severity::Critical,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum LineEndingArg {
    Lf,
    Crlf,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("no-watermark: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let lang = cli
        .global
        .lang
        .as_deref()
        .and_then(Lang::parse)
        .unwrap_or_else(Lang::detect);
    let profile = build_profile(&cli.global);
    let style = Style {
        color: match cli.global.color {
            ColorArg::Always => true,
            ColorArg::Never => false,
            ColorArg::Auto => io::stdout().is_terminal(),
        },
    };

    let command = cli.command.unwrap_or(Command::Clean(CleanArgs::default()));

    match command {
        Command::Clean(args) => cmd_clean(args, &cli.global, &profile, lang, &style),
        Command::Scan(args) => cmd_scan(args, &cli.global, lang, &style),
        Command::Check(args) => cmd_check(args, &cli.global, &profile, lang, &style),
        Command::Reveal(args) => cmd_reveal(args),
        Command::Decode(args) => cmd_decode(args, &cli.global, lang, &style),
    }
}

/// Turn the preset plus any `--keep` / `--remove` overrides into a profile.
fn build_profile(g: &GlobalArgs) -> Profile {
    let mut p = Profile::from_preset(g.profile.into());

    for c in &g.remove {
        set_category(&mut p, (*c).into(), true);
    }
    // `--keep` is applied last so it always wins over `--remove`.
    for c in &g.keep {
        set_category(&mut p, (*c).into(), false);
    }

    if g.no_preserve_emoji {
        p.preserve_emoji_joiners = false;
    }
    if g.no_preserve_script {
        p.preserve_script_joiners = false;
    }
    p.line_ending = g.line_endings.map(|e| match e {
        LineEndingArg::Lf => nowm_core::LineEnding::Lf,
        LineEndingArg::Crlf => nowm_core::LineEnding::Crlf,
    });
    p
}

fn set_category(p: &mut Profile, c: Category, on: bool) {
    match c {
        Category::Tag => p.remove_tags = on,
        Category::Invisible | Category::Deprecated => p.remove_invisible = on,
        Category::Bidi => p.remove_bidi = on,
        Category::VariationSelector => p.remove_variation_selectors = on,
        Category::Space => p.normalize_spaces = on,
        Category::Typography => p.ascii_typography = on,
        Category::Homoglyph => p.fold_homoglyphs = on,
    }
}

// ---------------------------------------------------------------------------
// commands
// ---------------------------------------------------------------------------

fn cmd_clean(
    args: CleanArgs,
    g: &GlobalArgs,
    profile: &Profile,
    lang: Lang,
    style: &Style,
) -> Result<ExitCode> {
    let sources = collect_sources(&args.input)?;

    if args.in_place {
        let mut modified = 0usize;
        for source in &sources {
            let Source::File(path) = source else {
                bail!("--in-place needs at least one file");
            };
            let original = read_source(source)?;
            let report = analyze(&original, profile);
            if report.cleaned == original {
                if !g.quiet {
                    println!(
                        "{}",
                        i18n::tf(lang, "cli.unchanged", &[&path.display().to_string()])
                    );
                }
                continue;
            }
            modified += 1;
            if args.dry_run {
                if !g.quiet {
                    println!(
                        "{}",
                        i18n::tf(lang, "cli.clean_ok", &[&path.display().to_string()])
                    );
                }
                continue;
            }
            if args.backup {
                let mut backup = path.clone().into_os_string();
                backup.push(".bak");
                let backup = PathBuf::from(backup);
                fs::copy(path, &backup)
                    .with_context(|| format!("writing backup {}", backup.display()))?;
                if !g.quiet {
                    println!(
                        "{}",
                        i18n::tf(lang, "cli.backup_written", &[&backup.display().to_string()])
                    );
                }
            }
            fs::write(path, &report.cleaned)
                .with_context(|| format!("writing {}", path.display()))?;
            if !g.quiet {
                println!(
                    "{}",
                    i18n::tf(lang, "cli.clean_ok", &[&path.display().to_string()])
                );
            }
        }
        if !g.quiet {
            println!(
                "{}, {}",
                i18n::tf(lang, "cli.files_scanned", &[&sources.len().to_string()]),
                i18n::tf(lang, "cli.files_modified", &[&modified.to_string()])
            );
            if args.dry_run {
                println!("{}", i18n::t(lang, "cli.dry_run"));
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Streaming mode: concatenate every source and emit one result.
    let mut cleaned = String::new();
    let mut worst: Option<Report> = None;
    for source in &sources {
        let report = analyze(&read_source(source)?, profile);
        cleaned.push_str(&report.cleaned);
        if worst
            .as_ref()
            .map(|w| w.score < report.score)
            .unwrap_or(true)
        {
            worst = Some(report);
        }
    }

    if g.json {
        let report = worst.expect("at least one source");
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(ExitCode::SUCCESS);
    }

    if args.dry_run {
        if !g.quiet {
            eprintln!("{}", i18n::t(lang, "cli.dry_run"));
            if let Some(r) = &worst {
                let mut buf = String::new();
                render::report(&mut buf, r, lang, style, None);
                eprint!("{buf}");
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    match &args.output {
        Some(path) => {
            fs::write(path, &cleaned).with_context(|| format!("writing {}", path.display()))?;
            if !g.quiet {
                eprintln!(
                    "{}",
                    i18n::tf(lang, "cli.wrote", &[&path.display().to_string()])
                );
            }
        }
        None => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(cleaned.as_bytes())?;
            stdout.flush()?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_scan(args: InputArgs, g: &GlobalArgs, lang: Lang, style: &Style) -> Result<ExitCode> {
    let sources = collect_sources(&args)?;
    let profile = Profile::scan();
    let mut reports = Vec::new();

    for source in &sources {
        let report = analyze(&read_source(source)?, &profile);
        reports.push((source.label(), report));
    }

    if g.json {
        let payload: Vec<_> = reports
            .iter()
            .map(|(label, r)| serde_json::json!({ "source": label, "report": r }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(ExitCode::SUCCESS);
    }

    let mut buf = String::new();
    for (i, (label, r)) in reports.iter().enumerate() {
        if i > 0 {
            buf.push('\n');
        }
        render::report(&mut buf, r, lang, style, Some(label));
    }
    if !g.quiet {
        buf.push('\n');
        buf.push_str(&style.dim(i18n::t(lang, "limits.body")));
        buf.push('\n');
    }
    print!("{buf}");
    Ok(ExitCode::SUCCESS)
}

fn cmd_check(
    args: CheckArgs,
    g: &GlobalArgs,
    profile: &Profile,
    lang: Lang,
    style: &Style,
) -> Result<ExitCode> {
    let sources = collect_sources(&args.input)?;
    let min: Severity = args.min_severity.into();
    let mut failed = false;
    let mut lines = String::new();

    for source in &sources {
        let report = analyze(&read_source(source)?, profile);
        if nowm_core::fails_check(&report, min) {
            failed = true;
        }
        lines.push_str(&render::short_line(&report, lang, style, &source.label()));
    }

    if g.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "failed": failed }))?
        );
    } else if !g.quiet {
        print!("{lines}");
        println!(
            "{}",
            if failed {
                i18n::t(lang, "cli.check_failed")
            } else {
                i18n::t(lang, "cli.check_passed")
            }
        );
    }

    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn cmd_reveal(args: InputArgs) -> Result<ExitCode> {
    let sources = collect_sources(&args)?;
    let mut stdout = io::stdout().lock();
    for source in &sources {
        stdout.write_all(visualize(&read_source(source)?).as_bytes())?;
    }
    stdout.flush()?;
    Ok(ExitCode::SUCCESS)
}

fn cmd_decode(args: InputArgs, g: &GlobalArgs, lang: Lang, style: &Style) -> Result<ExitCode> {
    let sources = collect_sources(&args)?;
    let profile = Profile::scan();
    let mut all = Vec::new();

    for source in &sources {
        let report = analyze(&read_source(source)?, &profile);
        for p in report.payloads {
            all.push((source.label(), p));
        }
    }

    if g.json {
        let payload: Vec<_> = all
            .iter()
            .map(|(label, p)| serde_json::json!({ "source": label, "payload": p }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(ExitCode::SUCCESS);
    }

    if all.is_empty() {
        if !g.quiet {
            println!("{}", i18n::t(lang, "report.no_findings"));
        }
        return Ok(ExitCode::SUCCESS);
    }

    println!("{}", style.bold(i18n::t(lang, "report.hidden_payloads")));
    for (label, p) in &all {
        println!(
            "{label}: {} @ {}",
            i18n::payload_kind(lang, p.kind),
            p.start_char
        );
        match &p.text {
            Some(text) => println!("  {text}"),
            None => println!("  {}", p.hex),
        }
    }
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------
// input handling
// ---------------------------------------------------------------------------

enum Source {
    Stdin,
    File(PathBuf),
}

impl Source {
    fn label(&self) -> String {
        match self {
            Source::Stdin => "<stdin>".to_string(),
            Source::File(p) => p.display().to_string(),
        }
    }
}

fn collect_sources(args: &InputArgs) -> Result<Vec<Source>> {
    if args.files.is_empty() {
        if io::stdin().is_terminal() {
            bail!("{}", i18n::t(Lang::detect(), "cli.no_input"));
        }
        return Ok(vec![Source::Stdin]);
    }
    Ok(args.files.iter().map(|p| Source::File(p.clone())).collect())
}

fn read_source(source: &Source) -> Result<String> {
    match source {
        Source::Stdin => {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf)?;
            decode_utf8(&buf, Path::new("<stdin>"))
        }
        Source::File(path) => {
            let buf = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            decode_utf8(&buf, path)
        }
    }
}

/// Reject non-UTF-8 rather than guessing: silently replacing invalid bytes
/// would corrupt the very file the user asked us to clean.
fn decode_utf8(bytes: &[u8], path: &Path) -> Result<String> {
    String::from_utf8(bytes.to_vec())
        .with_context(|| format!("{} is not valid UTF-8", path.display()))
}
