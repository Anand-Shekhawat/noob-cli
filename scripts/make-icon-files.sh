#!/usr/bin/env bash
# Turns assets/noob-icon.png into the icon files each computer wants:
#   assets/Noob.icns  (Mac)    and    assets/noob.ico  (Windows)
# Run on a Mac (it uses Apple's sips and iconutil).
set -euo pipefail
cd "$(dirname "$0")/.."

swift scripts/make-icon.swift assets/noob-icon.png

set_dir="$(mktemp -d)/Noob.iconset"
mkdir -p "$set_dir"
for px in 16 32 128 256 512; do
  sips -z "$px" "$px" assets/noob-icon.png --out "$set_dir/icon_${px}x${px}.png" >/dev/null
  sips -z $((px * 2)) $((px * 2)) assets/noob-icon.png --out "$set_dir/icon_${px}x${px}@2x.png" >/dev/null
done
iconutil -c icns "$set_dir" -o assets/Noob.icns

# A Windows .ico can hold PNG pictures directly: a small header, then the PNGs.
python3 - "$set_dir" <<'PY'
import struct, sys, pathlib
folder = pathlib.Path(sys.argv[1])
sizes = [16, 32, 48, 256]
pictures = []
for px in sizes:
    source = folder / f"icon_{px}x{px}.png"
    if not source.exists():  # 48 isn't in the Mac set, so make it
        import subprocess
        subprocess.run(["sips", "-z", str(px), str(px), "assets/noob-icon.png", "--out", str(source)], check=True, capture_output=True)
    pictures.append(source.read_bytes())
header = struct.pack("<HHH", 0, 1, len(pictures))
offset = 6 + 16 * len(pictures)
entries = b""
for px, data in zip(sizes, pictures):
    entries += struct.pack("<BBBBHHII", px % 256, px % 256, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
pathlib.Path("assets/noob.ico").write_bytes(header + entries + b"".join(pictures))
PY
echo "Wrote assets/Noob.icns and assets/noob.ico"
