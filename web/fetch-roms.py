#!/usr/bin/env python3
"""Populate web/roms/ from the manifest in web/roms.json.

Entries with a `fixture` path are copied from this repository; the rest are
downloaded from the URL in the manifest (optionally extracting `zip_member`
from a zip) and verified against the pinned `sha256`. Already-present files
with a matching hash are skipped, so re-running is cheap.

Usage: python3 web/fetch-roms.py [--out DIR]
"""
import hashlib
import io
import json
import shutil
import sys
import urllib.request
import zipfile
from pathlib import Path

WEB = Path(__file__).resolve().parent
ROOT = WEB.parent


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fetch(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "rgba-fetch-roms"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def main() -> int:
    out = WEB / "roms"
    if len(sys.argv) >= 3 and sys.argv[1] == "--out":
        out = Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)

    manifest = json.loads((WEB / "roms.json").read_text())
    failed = 0
    for group in manifest["groups"]:
        for rom in group["roms"]:
            dest = out / rom["file"]
            if "fixture" in rom:
                shutil.copyfile(ROOT / rom["fixture"], dest)
                print(f"copied   {rom['file']}  <- {rom['fixture']}")
                continue

            if dest.exists() and sha256(dest.read_bytes()) == rom["sha256"]:
                print(f"cached   {rom['file']}")
                continue

            print(f"download {rom['file']}  <- {rom['url']}")
            try:
                data = fetch(rom["url"])
                if "zip_member" in rom:
                    with zipfile.ZipFile(io.BytesIO(data)) as z:
                        data = z.read(rom["zip_member"])
            except Exception as e:  # noqa: BLE001 - report and keep going
                print(f"  !! failed: {e}")
                failed += 1
                continue

            digest = sha256(data)
            if digest != rom["sha256"]:
                print(f"  !! sha256 mismatch for {rom['id']}: got {digest}")
                failed += 1
                continue
            dest.write_bytes(data)
            print(f"  ok ({len(data)} bytes)")

    if failed:
        print(f"{failed} ROM(s) failed", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
