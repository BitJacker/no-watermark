# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/).

## [1.0.1] - 2026-08-28

### Fixed

Cleaning is meant to be idempotent, and a randomised test added in this
release found three separate ways it was not. Each came from the same root
cause: the analysis was looking at text that differed from the text it
emitted.

- **NFKC expansions were folded one run late.** `U+00BE` normalises to
  `3⁄4`, introducing a `FRACTION SLASH` that the typography rule rewrites to
  `/`. Because normalisation ran after the character pass, the slash survived
  the first run and only disappeared on the second. Normalisation now runs
  before the text is inspected.
- **Removing a character could move a word boundary.** Whether a confusable
  letter gets folded depends on the word around it, and deleting an invisible
  character or rewriting `U+201A` into an apostrophe changed where that word
  started and ended. A Cyrillic `Е` left alone on the first run was folded on
  the second. Format characters are now transparent to word boundaries, and
  apostrophes and hyphens no longer count as word characters.
- **Removing a character could enable a canonical reordering.** A BOM between
  two Arabic combining marks keeps them apart; delete it and NFKC sorts them
  by combining class. Normalisation now also runs after cleaning.

### Added

- Randomised invariant tests over inputs drawn from the codepoint ranges this
  crate reasons about: `scan` never edits, cleaning is idempotent for every
  profile, `safe` preserves every visible glyph, and reported positions always
  point at the character they claim.
- CI validates the packaging sources on every run, so an error in the WiX,
  Debian or AppImage definitions surfaces in the pull request rather than
  during a release.

### Changed

- The MSI no longer ships the WiX symbol file alongside the installer.

## [1.0.0] - 2026-08-28

First release.

### Added

- **Detection engine** (`nowm-core`) covering Unicode tag characters, zero-width
  and format characters, bidirectional controls, variation selectors,
  confusable letters, whitespace look-alikes and typography.
- **Hidden payload recovery** for three carrier schemes: the Unicode Tags
  block, variation-selector byte runs, and zero-width binary runs.
- **Context-aware safety**: emoji joiners, Arabic/Persian/Indic joiners and
  genuine non-Latin words are recognised and preserved.
- **Four profiles**: `scan`, `safe`, `standard`, `aggressive`, each rule
  individually overridable.
- **Stylometric signals** reported without claiming they prove anything,
  including the `U+202F` next to em dash pattern.
- **Command line tool** with `clean`, `scan`, `check`, `reveal` and `decode`
  subcommands, JSON output and CI-friendly exit codes.
- **Desktop application** built with egui: live cleaning, invisible-character
  reveal, findings table, payload decoder and optional clipboard watching.
- **English and Italian** throughout, switchable at runtime, defaulting to the
  system locale.
- **Packaging** for Windows (MSI, portable zip) and Linux (deb, rpm, AppImage,
  portable tarball), all produced by GitHub Actions on tag.

[1.0.1]: https://github.com/BitJacker/no-watermark/releases/tag/v1.0.1
[1.0.0]: https://github.com/BitJacker/no-watermark/releases/tag/v1.0.0
