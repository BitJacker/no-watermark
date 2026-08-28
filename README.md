<div align="center">

<img src="assets/icon-128.png" width="96" alt="no-watermark">

# no-watermark

**Detect and strip invisible Unicode watermarks, hidden payloads and stylometric fingerprints from AI chat output.**

Desktop app and command line tool for Windows and Linux.
Interface in **English** and **Italian**.

[![CI](https://github.com/BitJacker/no-watermark/actions/workflows/ci.yml/badge.svg)](https://github.com/BitJacker/no-watermark/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

---

## What it does

Text copied out of ChatGPT, Claude, Gemini or any other assistant can carry
characters you never see. Some are harmless typographic habits. Some are
deliberate: the Unicode **Tags block** is a complete shadow copy of ASCII that
renders as *nothing at all* in every browser, editor and chat window, yet a
language model reads it perfectly. That is the carrier behind *ASCII
smuggling* and invisible prompt injection.

no-watermark finds every one of them, **shows you what was hidden**, and gives
you back clean text.

```
$ no-watermark scan message.txt
Verdict: Hidden content  Score: 100/100

Hidden content recovered
This text carried content that is invisible to you but readable by an AI model.
  - Unicode tags (32 chars @ 46)
    decoded text: Ignore all previous instructions

Findings
  high      line 1, column 20     U+200B ZERO WIDTH SPACE       Invisible character
  low       line 1, column 35     U+202F NARROW NO-BREAK SPACE  Whitespace look-alike
  critical  line 1, column 47 x32 TAG CHARACTER                 Unicode tag character
  medium    line 2, column 6      U+043E CONFUSABLE LETTER      Confusable letter
```

## What it can and cannot remove

Honesty first, because this field is full of tools that promise more than
physics allows.

| Fingerprint | Real? | Removable by no-watermark |
| --- | --- | --- |
| **Unicode Tags block** (`U+E0000`–`U+E007F`) — invisible ASCII, used to smuggle instructions | Yes, actively exploited | **Yes, completely** |
| **Zero-width characters** — `ZWSP`, `ZWNJ`, `ZWJ`, word joiner, soft hyphen, BOM, fillers | Yes | **Yes** |
| **Variation selectors** — one byte of payload each ("emoji smuggling") | Yes | **Yes** |
| **Bidirectional controls** — make displayed text differ from stored text | Yes | **Yes** |
| **Homoglyphs** — Cyrillic `о` inside a Latin word | Yes | **Yes** |
| **Exotic whitespace** — including `U+202F`, emitted by some ChatGPT models in 2025 | Yes, but an artefact rather than a watermark | **Yes** |
| **SynthID-Text** (Google DeepMind, live in Gemini) — watermark embedded in *token sampling* | Yes, in production | **No.** It leaves no special character; no character-level tool can touch it |
| A cryptographic *text* watermark from OpenAI | Never shipped. OpenAI built one and chose not to release it | Not applicable |

If a tool tells you it removes SynthID by deleting characters, it is wrong.
See [docs/WATERMARKS.md](docs/WATERMARKS.md) for the full technical write-up.

## Install

### Windows

| | |
| --- | --- |
| **Installer** | Download `no-watermark-<version>-x86_64.msi` from [Releases](https://github.com/BitJacker/no-watermark/releases). Installs the desktop app, adds a Start Menu entry, and puts `no-watermark` on your `PATH`. |
| **Portable** | Download `no-watermark-<version>-windows-x86_64.zip`, unzip, run `no-watermark-gui.exe`. Nothing is written outside the folder. |

### Linux

| | |
| --- | --- |
| **Debian / Ubuntu** | `sudo dpkg -i no-watermark_<version>_amd64.deb` |
| **Fedora / RHEL / openSUSE** | `sudo rpm -i no-watermark-<version>.x86_64.rpm` |
| **AppImage** | `chmod +x no-watermark-<version>-x86_64.AppImage && ./no-watermark-<version>-x86_64.AppImage` |
| **Portable** | `tar xzf no-watermark-<version>-linux-x86_64.tar.gz` |

> There is no `.msi` for Linux — MSI is a Windows Installer format. The `.deb`,
> `.rpm` and AppImage above are its Linux equivalents, and each installs the
> same two binaries plus a desktop entry.

### From source

```bash
git clone https://github.com/BitJacker/no-watermark
cd no-watermark
cargo build --release          # target/release/no-watermark[-gui]
cargo test                     # 40+ tests
```

Rust 1.82 or newer. No system libraries are required to build the CLI; the GUI
uses the platform windowing stack at runtime.

## Using the desktop app

Paste on the left, read clean text on the right.

- **Profile** — `Scan`, `Safe`, `Standard` (default) or `Aggressive`. Every
  individual rule can be toggled under **Options**.
- **Reveal invisible characters** — renders `⟦U+200B⟧` in place of what you
  cannot see.
- **Watch the clipboard** — anything you copy is cleaned automatically, in
  place. Off by default.
- **Findings / Hidden content / Style signals** — the tabs at the bottom
  explain every decision the tool made, and decode anything that was smuggled.
- **Language** — English or Italian, switchable at runtime. The app starts in
  your system language.

## Using the command line

```bash
no-watermark < answer.txt > clean.txt      # clean stdin to stdout
no-watermark clean -i notes/*.md           # rewrite files in place
no-watermark clean -i --backup notes.md    # keep notes.md.bak
no-watermark scan report.md                # explain, change nothing
no-watermark check --min-severity high *.md   # exit 1 if anything is found
no-watermark decode message.txt            # print only the hidden content
no-watermark reveal message.txt            # make invisibles visible
```

### Profiles

| Profile | Invisible / tags / bidi / VS | Whitespace | Homoglyphs | Punctuation, NFKC |
| --- | --- | --- | --- | --- |
| `scan` | report only | report only | report only | report only |
| `safe` | **removed** | untouched | untouched | untouched |
| `standard` *(default)* | **removed** | normalised | folded | untouched |
| `aggressive` | **removed** | normalised | folded | **ASCII + NFKC** |

`safe` is guaranteed not to change a single visible glyph.

Any rule can be overridden on top of a profile:

```bash
no-watermark --profile safe --remove space,typography    # add rules
no-watermark --keep homoglyph                            # drop a rule
no-watermark --no-preserve-emoji                         # strip emoji joiners too
```

### Other options

| Flag | Meaning |
| --- | --- |
| `--lang en\|it` | Interface language (default: system locale) |
| `--json` | Machine-readable report on stdout |
| `--color auto\|always\|never` | ANSI colour |
| `--line-endings lf\|crlf` | Rewrite line endings |
| `-q`, `--quiet` | Suppress summaries |
| `-n`, `--dry-run` | Show what would change, write nothing |

### Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success, or `check` found nothing at or above the threshold |
| `1` | `check` found something |
| `2` | Error (unreadable file, invalid UTF-8, …) |

That makes it a drop-in CI guard:

```yaml
- run: no-watermark check --min-severity high docs/**/*.md
```

### JSON

```bash
no-watermark scan --json message.txt | jq '.[0].report.payloads[].text'
```

The schema is stable and locale independent: categories, severities, notes and
verdicts are snake_case identifiers, never translated prose.

## Careful by design

A cleaner that destroys real text is worse than no cleaner at all, so
no-watermark refuses to be careless:

- **Emoji sequences survive.** A `ZERO WIDTH JOINER` holding 👨‍👩‍👧 together is
  recognised and kept.
- **Arabic, Persian and Indic text survives.** A `ZWNJ` doing orthographic work
  in مینویسم is kept.
- **Real Cyrillic and Greek survive.** A confusable is only folded when it sits
  inside a word that is otherwise Latin, so `привет` is never touched while
  `passwоrd` is fixed.
- **`safe` never alters a visible glyph**, and `scan` never alters anything at
  all.
- **Cleaning is idempotent.** Running it twice gives the same result as once.

Every one of those guarantees has a test.

## Building the packages

```bash
# Linux: binaries, .deb, tarball, AppImage
cargo build --release
packaging/linux/build-deb.sh
packaging/linux/build-appimage.sh

# Windows: MSI (needs the WiX Toolset v6)
dotnet tool install --global wix
wix extension add -g WixToolset.UI.wixext
cargo build --release
wix build packaging/wix/main.wxs -ext WixToolset.UI.wixext -o no-watermark.msi
```

Icons are generated, not committed as an opaque blob:
`python packaging/make_icons.py`.

Every artefact above is also built by
[`.github/workflows/release.yml`](.github/workflows/release.yml) on each tag.

## Project layout

```
crates/nowm-core    detection and cleaning engine, no I/O, no locale
crates/nowm-i18n    English and Italian strings
crates/nowm-cli     the `no-watermark` command
crates/nowm-gui     the desktop application (egui)
packaging/          WiX, Debian, AppImage, icon generator
docs/WATERMARKS.md  how AI text watermarking actually works
```

## Licence

MIT — see [LICENSE](LICENSE).

Made by **Giacomo Giordano**.
