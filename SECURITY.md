# Security policy

## Reporting a vulnerability

Please report security issues privately through
[GitHub's private vulnerability reporting](https://github.com/BitJacker/no-watermark/security/advisories/new)
rather than in a public issue.

Include the input that triggers the problem where possible. Since the inputs
this tool handles are usually invisible, `no-watermark scan --json` output or
a base64 of the file is more useful than a paste.

## What counts as a vulnerability here

no-watermark is often used as a **sanitiser in front of a language model**, so
the interesting failures are the ones that let something through:

- Input containing a hidden payload that `scan` reports as clean.
- A carrier scheme the decoder misses entirely.
- Cleaned output that still contains characters the active profile should have
  removed.
- A crafted input that makes the tool panic, hang, or consume unbounded memory.
- Cleaning that corrupts legitimate text — an emoji sequence broken, a Persian
  word mangled, a genuinely Cyrillic document Latinised.

That last one matters as much as the others: a cleaner people stop trusting is
a cleaner people stop using.

## Scope and expectations

- no-watermark reads text and writes text. It makes no network connections at
  any point, and the desktop application only touches the clipboard when you
  turn that option on.
- **Sanitising is not a complete defence against prompt injection.** Stripping
  invisible characters removes one carrier. Visible text can carry an
  injection just as well, and no character-level tool addresses that. Treat
  cleaned untrusted text as untrusted.
- no-watermark cannot remove statistical watermarks such as SynthID-Text. That
  is a property of how those schemes work, not a bug. See
  [docs/WATERMARKS.md](docs/WATERMARKS.md).

## Supported versions

The latest release is supported. Fixes land on `main` and go out in the next
tagged release.
