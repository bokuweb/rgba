#!/usr/bin/env python3
"""Send a multiboot image to a GBA through Optimize's "GBA BOOT CABLE USB"
(USB 0BFE:3000), the cable bundled with GBA programming books around 2003.

The protocol follows SegaraRai's optusbx (https://github.com/SegaraRai/optusbx),
a rewrite of optusb. The cable runs the multiboot handshake with the GBA
itself; the PC only checks the GBA is ready and streams the image.

Needs libusb (brew install libusb); it is loaded with ctypes, so nothing is
compiled. Set LIBUSB to the dylib's path if it is not in Homebrew's prefix.

Usage: optusb-send.py IMAGE.gba [--wait MS] [--timeout MS]
Turn the GBA on with no cartridge and the cable plugged in, then run this
while it shows the white boot screen.
"""
import argparse
import ctypes
import os
import sys
import time

VENDOR_ID, PRODUCT_ID = 0x0BFE, 0x3000
EP_COMMAND, EP_DATA = 0x01, 0x02
EP_OUT, EP_IN = 0x00, 0x80
CMD_WRITE, CMD_STATUS = 0x02, 0x03
STATUS_POWER, STATUS_READY = 1 << 0, 1 << 1
NEGOTIATION_MAGIC = 0xFEDCBA98
MAX_PROGRAM = 256 * 1024


class SendError(Exception):
    pass


def load_libusb():
    candidates = [os.environ.get("LIBUSB"), "/opt/homebrew/opt/libusb/lib/libusb-1.0.dylib",
                  "/usr/local/opt/libusb/lib/libusb-1.0.dylib", "libusb-1.0.so.0"]
    for path in filter(None, candidates):
        try:
            lib = ctypes.CDLL(path)
            break
        except OSError:
            continue
    else:
        raise SendError("libusb not found; run: brew install libusb (or set LIBUSB)")
    lib.libusb_init.argtypes = [ctypes.c_void_p]
    lib.libusb_open_device_with_vid_pid.restype = ctypes.c_void_p
    lib.libusb_open_device_with_vid_pid.argtypes = [ctypes.c_void_p, ctypes.c_uint16, ctypes.c_uint16]
    lib.libusb_claim_interface.argtypes = [ctypes.c_void_p, ctypes.c_int]
    lib.libusb_release_interface.argtypes = [ctypes.c_void_p, ctypes.c_int]
    lib.libusb_close.argtypes = [ctypes.c_void_p]
    lib.libusb_exit.argtypes = [ctypes.c_void_p]
    lib.libusb_bulk_transfer.argtypes = [ctypes.c_void_p, ctypes.c_ubyte, ctypes.c_void_p, ctypes.c_int,
                                         ctypes.POINTER(ctypes.c_int), ctypes.c_uint]
    lib.libusb_error_name.restype = ctypes.c_char_p
    lib.libusb_error_name.argtypes = [ctypes.c_int]
    return lib


class Cable:
    def __init__(self, lib, wait_ms, timeout_ms):
        self.lib, self.wait, self.timeout = lib, wait_ms / 1000, timeout_ms
        self.check(lib.libusb_init(None), "libusb_init failed")
        self.dev = lib.libusb_open_device_with_vid_pid(None, VENDOR_ID, PRODUCT_ID)
        if not self.dev:
            raise SendError("GBA BOOT CABLE USB (0BFE:3000) not found")
        self.check(lib.libusb_claim_interface(self.dev, 0), "cannot claim the cable's interface")

    def close(self):
        self.lib.libusb_release_interface(self.dev, 0)
        self.lib.libusb_close(self.dev)
        self.lib.libusb_exit(None)

    def check(self, rc, what):
        if rc < 0:
            raise SendError(f"{what} ({self.lib.libusb_error_name(rc).decode()})")

    def bulk(self, ep, buf):
        done = ctypes.c_int(0)
        rc = self.lib.libusb_bulk_transfer(self.dev, ep, buf, len(buf), ctypes.byref(done), self.timeout)
        self.check(rc, "USB receive failed" if ep & EP_IN else "USB send failed")
        if done.value != len(buf):
            raise SendError("USB transfer was cut short")

    def send(self, ep, data):
        self.bulk(EP_OUT | ep, (ctypes.c_ubyte * len(data)).from_buffer_copy(bytes(data)))

    def receive_word(self):
        buf = (ctypes.c_ubyte * 4)()
        self.bulk(EP_IN | EP_DATA, buf)
        return int.from_bytes(bytes(buf), "little")

    def read_status(self, n):
        """Reads status register n from the cable."""
        self.send(EP_COMMAND, [CMD_STATUS, n])
        time.sleep(self.wait)
        return self.receive_word()

    def write_status(self, n, code):
        """Writes code to status register n and returns the cable's reply."""
        self.send(EP_COMMAND, [CMD_STATUS, 0x80 | n, *code.to_bytes(4, "little")])
        time.sleep(self.wait)
        return self.receive_word()

    def negotiate(self):
        if self.read_status(1) == 0x12345678:
            raise SendError("negotiation error [1]")
        self.write_status(2, NEGOTIATION_MAGIC)
        if self.read_status(2) != NEGOTIATION_MAGIC:
            raise SendError("negotiation error [2]")
        self.write_status(2, 0)
        status = self.read_status(0)
        if not (status & STATUS_POWER and status & STATUS_READY):
            raise SendError(f"GBA not ready (status {status:08x}). Turn it on with no cartridge"
                            " and run this on the white boot screen.")
        if self.read_status(1) == 0x12345678:
            raise SendError("negotiation error [4]")

    def send_program(self, image):
        aligned = image + bytes(-len(image) % 4)
        words = len(aligned) // 4
        self.send(EP_COMMAND, [CMD_WRITE, 1, 0])  # one word follows: the size
        time.sleep(self.wait)
        self.send(EP_DATA, len(image).to_bytes(4, "little"))
        time.sleep(self.wait)
        self.send(EP_COMMAND, [CMD_WRITE, words & 0xFF, words >> 8 & 0xFF])
        time.sleep(self.wait)
        self.send(EP_DATA, aligned)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("image")
    ap.add_argument("--wait", type=int, default=30, help="pause after each command, in ms (default 30)")
    ap.add_argument("--timeout", type=int, default=5000, help="USB timeout in ms (default 5000)")
    args = ap.parse_args()

    image = open(args.image, "rb").read()
    if not image:
        sys.exit("optusb-send: the image is empty")
    if len(image) > MAX_PROGRAM:
        sys.exit("optusb-send: the image is larger than 256 KiB")

    try:
        cable = Cable(load_libusb(), args.wait, args.timeout)
        try:
            cable.negotiate()
            print(f"sending {len(image)} bytes...")
            cable.send_program(image)
            print("done")
        finally:
            cable.close()
    except SendError as e:
        sys.exit(f"optusb-send: {e}")


if __name__ == "__main__":
    main()
