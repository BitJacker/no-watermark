# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/).

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

[1.0.0]: https://github.com/BitJacker/no-watermark/releases/tag/v1.0.0
