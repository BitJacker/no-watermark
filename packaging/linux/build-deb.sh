#!/usr/bin/env bash
# Build a Debian package containing both no-watermark binaries.
#
# The package is assembled by hand with dpkg-deb rather than through a cargo
# plugin: the layout is small enough to read in one screen, and it keeps the
# release pipeline dependent on nothing but a standard Debian toolchain.
#
#   packaging/linux/build-deb.sh [VERSION] [ARCH]
#
# Run from the repository root, after:
#   cargo build --release -p nowm-cli -p nowm-gui

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)}"
ARCH="${2:-$(dpkg --print-architecture 2>/dev/null || echo amd64)}"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
BIN_DIR="$TARGET_DIR/release"
OUT="dist"
PKG="no-watermark_${VERSION}_${ARCH}"

# Stage inside a real Linux filesystem. Building straight from a Windows
# drive mount (or any filesystem without POSIX permissions) makes dpkg-deb
# reject the control directory for being mode 0777.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
STAGE="$WORK/$PKG"

for bin in no-watermark no-watermark-gui; do
    if [ ! -x "$BIN_DIR/$bin" ]; then
        echo "missing $BIN_DIR/$bin - run: cargo build --release" >&2
        exit 1
    fi
done

mkdir -p "$STAGE/DEBIAN" \
         "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/doc/no-watermark" \
         "$STAGE/usr/share/metainfo"

install -m 0755 "$BIN_DIR/no-watermark"     "$STAGE/usr/bin/no-watermark"
install -m 0755 "$BIN_DIR/no-watermark-gui" "$STAGE/usr/bin/no-watermark-gui"
install -m 0644 packaging/linux/no-watermark.desktop \
    "$STAGE/usr/share/applications/no-watermark.desktop"
install -m 0644 LICENSE   "$STAGE/usr/share/doc/no-watermark/copyright"
install -m 0644 README.md "$STAGE/usr/share/doc/no-watermark/README.md"
install -m 0644 packaging/linux/no-watermark.metainfo.xml     "$STAGE/usr/share/metainfo/io.github.bitjacker.no-watermark.metainfo.xml"

for size in 16 32 48 64 128 256 512; do
    dir="$STAGE/usr/share/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$dir"
    install -m 0644 "assets/icon-${size}.png" "$dir/no-watermark.png"
done

INSTALLED_KB=$(du -ks "$STAGE" | cut -f1)

cat > "$STAGE/DEBIAN/control" <<EOF
Package: no-watermark
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Maintainer: Giacomo Giordano <noreply@users.noreply.github.com>
Installed-Size: $INSTALLED_KB
Depends: libc6 (>= 2.34), libgcc-s1
Recommends: libx11-6, libxkbcommon0, libgl1
Homepage: https://github.com/BitJacker/no-watermark
Description: Strip invisible Unicode watermarks from AI chat output
 no-watermark finds and removes the character-level fingerprints that end up
 in text copied out of an AI assistant: Unicode tag characters, zero-width
 and bidirectional format characters, variation selectors, confusable
 letters and exotic whitespace.
 .
 It also decodes content hidden with those characters, so an invisible
 prompt-injection payload can be read instead of merely deleted.
 .
 The package ships a command line tool (no-watermark) and a desktop
 application (no-watermark-gui). Both speak English and Italian.
EOF

# md5sums are optional but every well-formed package has them.
( cd "$STAGE" && find usr -type f -print0 \
    | xargs -0 md5sum > DEBIAN/md5sums )

mkdir -p "$OUT"
dpkg-deb --root-owner-group --build "$STAGE" "$OUT/$PKG.deb" >/dev/null
echo "$OUT/$PKG.deb"
