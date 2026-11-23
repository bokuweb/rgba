#!/usr/bin/env python3
"""Fill in the cartridge header: boot logo (copied from another ROM), title,
game code and complement checksum. Usage: fixrom.py ROM LOGO_SOURCE_ROM [--multiboot]

With --multiboot the image is also zero-padded to a multiple of 16 bytes,
because multiboot senders transfer it in 16-byte blocks and may refuse other
sizes."""
import sys

multiboot = "--multiboot" in sys.argv[3:]
rom = bytearray(open(sys.argv[1], "rb").read())
rom[0x04:0xA0] = open(sys.argv[2], "rb").read()[0x04:0xA0]
rom[0xA0:0xAC] = b"FLAPPYENAGA\0"
rom[0xAC:0xB0] = b"BFRE"
rom[0xB0:0xB2] = b"00"
rom[0xB2] = 0x96
rom[0xBD] = (-sum(rom[0xA0:0xBD]) - 0x19) & 0xFF
if multiboot:
    rom += bytes(-len(rom) % 16)
open(sys.argv[1], "wb").write(rom)
