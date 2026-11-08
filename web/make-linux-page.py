#!/usr/bin/env python3
"""Write the /linux/ page: play.html pinned to the r2-gba Linux ROM.

The page lives one directory below play.html, so it gets <base href="../">
and every relative URL (play.js, pkg/, roms.json, roms/) keeps resolving
against the site root. play.js reads <meta name="rgba-rom"> to pick the
cartridge, which brings the `psram` and `turbo` flags from roms.json.

Usage: python3 web/make-linux-page.py OUT_DIR   (writes OUT_DIR/index.html)
"""
import sys
from pathlib import Path

WEB = Path(__file__).resolve().parent


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    html = (WEB / "play.html").read_text()
    head = (
        '<head>\n<base href="../">\n'
        '<meta name="rgba-rom" content="linux">'
    )
    for old, new in [
        ("<head>", head),
        ("<title>rgba // play</title>", "<title>rgba // linux</title>"),
        (
            'content="rgba play mode: the Rust GBA emulator running inside a front-view handheld drawn in HTML and CSS."',
            'content="Linux 6.1 booting on a Game Boy Advance: bokuweb/r2\'s RISC-V core running in rgba, the Rust GBA emulator, compiled to WebAssembly."',
        ),
    ]:
        if html.count(old) != 1:
            print(f"play.html no longer contains {old!r} exactly once", file=sys.stderr)
            return 1
        html = html.replace(old, new)
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    (out / "index.html").write_text(html)
    print(f"wrote {out / 'index.html'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
