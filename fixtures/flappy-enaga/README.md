# Flappy Enaga

A one-button flyer for the Game Boy Advance starring a shimaenaga (a long-tailed tit), and the default ROM of the web debugger. Press **A** or **B** to flap and **Start** to pause.

The code is adapted from Tim Ullrich's [flappybird-gba](https://github.com/tullrich/flappybird-gba) (MIT, see [`LICENSE.txt`](LICENSE.txt)). That project ships the original game's sprites and sound effects, and none of them are used here. All art and sound are drawn by [`tools/gen_assets.py`](tools/gen_assets.py) and released under the same MIT terms.

Changes from upstream:

- New art and sound, generated at build time.
- Two column pairs on screen instead of one, and a wider gap.
- An on-screen score, a fall after dying, and random columns each game.
- A or B flaps. The L and R debug moves are gone.
- Builds with LLVM instead of devkitARM and grit, using the small runtime in [`rt/`](rt).

## Building

```bash
./build.sh
```

This needs Homebrew's `llvm@21` and `lld@21` and Python 3. Set `LLVM` and `LLD` to use other install paths. The ROM is committed, so the Pages build does not need this toolchain. The boot logo in the cartridge header is copied from `../hello/hello.gba`.
