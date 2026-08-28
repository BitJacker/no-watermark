#!/usr/bin/env bash
# Build a portable AppImage of the no-watermark desktop application.
#
#   packaging/linux/build-appimage.sh [VERSION]
#
# Run from the repository root, after:
#   cargo build --release -p nowm-cli -p nowm-gui
#
# appimagetool is downloaded once into target/tools if it is not already on
# PATH. FUSE is not required: the tool is invoked in extract-and-run mode.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)}"
ARCH="${ARCH:-x86_64}"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
BIN_DIR="$TARGET_DIR/release"
TOOLS="$TARGET_DIR/tools"
OUT="dist"

for bin in no-watermark no-watermark-gui; do
    if [ ! -x "$BIN_DIR/$bin" ]; then
        echo "missing $BIN_DIR/$bin - run: cargo build --release" >&2
        exit 1
    fi
done

# The AppDir must live on a filesystem with POSIX permissions.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
APPDIR="$WORK/no-watermark.AppDir"

mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/metainfo" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"

install -m 0755 "$BIN_DIR/no-watermark"     "$APPDIR/usr/bin/no-watermark"
install -m 0755 "$BIN_DIR/no-watermark-gui" "$APPDIR/usr/bin/no-watermark-gui"
install -m 0644 packaging/linux/no-watermark.desktop \
    "$APPDIR/usr/share/applications/no-watermark.desktop"
install -m 0644 packaging/linux/no-watermark.metainfo.xml \
    "$APPDIR/usr/share/metainfo/io.github.bitjacker.no-watermark.metainfo.xml"
install -m 0644 assets/icon-256.png \
    "$APPDIR/usr/share/icons/hicolor/256x256/apps/no-watermark.png"

# AppImage expects the desktop file and icon at the AppDir root as well.
cp "$APPDIR/usr/share/applications/no-watermark.desktop" "$APPDIR/no-watermark.desktop"
cp assets/icon-256.png "$APPDIR/no-watermark.png"

# Running the AppImage with arguments runs the CLI; without, the GUI. That
# makes a single portable file useful from both a launcher and a terminal.
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="$HERE/usr/bin:$PATH"
if [ "$#" -gt 0 ]; then
    exec "$HERE/usr/bin/no-watermark" "$@"
fi
exec "$HERE/usr/bin/no-watermark-gui"
EOF
chmod 0755 "$APPDIR/AppRun"

APPIMAGETOOL="$(command -v appimagetool || true)"
if [ -z "$APPIMAGETOOL" ]; then
    mkdir -p "$TOOLS"
    APPIMAGETOOL="$TOOLS/appimagetool"
    if [ ! -x "$APPIMAGETOOL" ]; then
        echo "downloading appimagetool..."
        curl -fsSL -o "$APPIMAGETOOL" \
            "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage"
        chmod +x "$APPIMAGETOOL"
    fi
fi

mkdir -p "$OUT"
OUTFILE="$OUT/no-watermark-${VERSION}-${ARCH}.AppImage"

APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$ARCH" \
    "$APPIMAGETOOL" --no-appstream "$APPDIR" "$OUTFILE"

chmod +x "$OUTFILE"
echo "$OUTFILE"
