# optusb-send

Sends a multiboot image to a Game Boy Advance through Optimize's **GBA BOOT CABLE USB** (USB ID `0BFE:3000`). Around 2003 this cable was bundled with Japanese GBA programming books. The protocol follows SegaraRai's [optusbx](https://github.com/SegaraRai/optusbx), a rewrite of optusb. The cable runs the multiboot handshake with the GBA itself. The PC checks that the GBA is ready, then streams the image.

Flappy Enaga's multiboot images have booted on a real GBA with this script on macOS. Both the C build ([`flappy-enaga_mb.gba`](../../fixtures/flappy-enaga)) and the Rust build ([`flappy-enaga-rs_mb.gba`](../../fixtures/flappy-enaga-rs)) work.

## Usage

```bash
brew install libusb
python3 tools/optusb-send/optusb-send.py fixtures/flappy-enaga/flappy-enaga_mb.gba
```

1. Take the cartridge out of the GBA and plug the cable into its link port.
2. Plug the cable into the computer. It should appear as `GBA BOOT CABLE USB` (`ioreg -p IOUSB` on macOS).
3. Turn the GBA on without holding any buttons. Holding R starts the cable's joystick mode, and holding START+SELECT+L starts its boot chip mode.
4. Run the script. It prints `done` and the game starts after the boot logo.

libusb is loaded with ctypes, so nothing is compiled. Set `LIBUSB` to the dylib's path if it is not under Homebrew's prefix. `--wait` and `--timeout` set the pause after each command and the USB timeout, in milliseconds.

## Troubleshooting

- **`GBA not ready (status 00000000)`**: the cable gets no power from the GBA's link port. Weak batteries are the usual cause: the power LED goes out right after the GBA is turned on. Use fresh batteries or an AC adapter, and check that the plug is fully in.
- **`not found`**: the cable did not enumerate on USB. Plug it straight into the computer, or try a USB 2.0 hub.
