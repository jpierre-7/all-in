#!/usr/bin/env bash
# Package a release build for itch.io (#71): the binary, assets/, a launcher,
# and the licences, zipped as dist/all-in-<platform>.zip.
#
#   tools/package.sh            # linux, from target/release/all-in
#   tools/package.sh windows target/x86_64-pc-windows-gnu/release/all-in.exe
#
# The game finds assets/ relative to its working directory (README: "Run it
# from the repo root"), so the launcher cd's to its own folder first. Do not
# run the bare binary from elsewhere.
set -euo pipefail

platform="${1:-linux}"
binary="${2:-target/release/all-in}"
name="all-in-$platform"
stage="dist/$name"

[ -f "$binary" ] || { echo "no binary at $binary; run cargo build --release first" >&2; exit 1; }

rm -rf "$stage" && mkdir -p "$stage"
cp "$binary" "$stage/"
cp -r assets "$stage/assets"
rm -f "$stage/assets/README.md"
cp README.md "$stage/README.md"

case "$platform" in
  linux)
    cat > "$stage/all-in.sh" <<'RUN'
#!/usr/bin/env bash
# Launch from this folder so the game can find assets/.
cd "$(dirname "$0")" && exec ./all-in "$@"
RUN
    chmod +x "$stage/all-in.sh" "$stage/all-in"
    ;;
  windows)
    printf '@echo off\r\ncd /d "%%~dp0"\r\nstart "" all-in.exe\r\n' > "$stage/all-in.bat"
    ;;
esac

cat > "$stage/CREDITS.txt" <<'TXT'
All In — HackRice 16, Rice University, September 2026
jpierre-7, HefKer, MEmshousen

Music: "Deadly Roulette" Kevin MacLeod (incompetech.com)
Licensed under Creative Commons: By Attribution 4.0 License
http://creativecommons.org/licenses/by/4.0/

Fonts: Barlow Condensed and Limelight, SIL Open Font License (see assets/fonts/).
TXT

mkdir -p dist
rm -f "dist/$name.zip"
# python's zipfile, so this works on a machine without the zip binary.
python3 - "$name" <<'PY'
import os, sys, zipfile
name = sys.argv[1]
with zipfile.ZipFile(f"dist/{name}.zip", "w", zipfile.ZIP_DEFLATED) as z:
    for root, _, files in os.walk(f"dist/{name}"):
        for f in files:
            path = os.path.join(root, f)
            info = zipfile.ZipInfo.from_file(path, os.path.relpath(path, "dist"))
            info.external_attr = (os.stat(path).st_mode & 0xFFFF) << 16  # keep +x
            with open(path, "rb") as fh:
                z.writestr(info, fh.read(), zipfile.ZIP_DEFLATED)
PY
du -h "dist/$name.zip"
