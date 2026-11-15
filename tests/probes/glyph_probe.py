#!/usr/bin/env python3
"""Native regression: graphical glyphs and background-only erased cells."""
import os
lines = [
    "GLYPH_AUDIT_READY",
    "\x1b[38;2;210;119;85m  \u2590\u259b\u2588\u2588\u2588\u259c\u258c",
    " \u259d\u259c\u2588\u2588\u2588\u2588\u2588\u259b\u2598",
    "   \u2598\u2598 \u259d\u259d\x1b[0m",
    "",
    "\u2588" * 30,
    "\u2588" * 30,
    "\u2580\u2584\u258c\u2590\u259b\u259c\u2599\u259f",
    "\u2500" * 50,
    "\u250f" + "\u2501" * 48 + "\u2513",
    "\u2503" + " " * 48 + "\u2503",
    "\u2517" + "\u2501" * 48 + "\u251b",
    "",
    "\x1b[48;2;55;55;55m\x1b[2K\x1b[0m",
    "\x1b[48;2;55;55;55m  This conversation is open in another app" + " " * 15 + "r to retry\x1b[K\x1b[0m",
    "\x1b[48;2;55;55;55m  Close it there and press R to continue here.\x1b[K\x1b[0m",
    "\x1b[48;2;55;55;55m\x1b[2K\x1b[0m",
]
os.write(1, ("\x1b[2J\x1b[H" + "\r\n".join(lines) + "\r\n").encode())

os.write(1, b"\x1b[48;2;55;55;55mTEXT_WITH_ERASE_TO_END\x1b[K\x1b[0m\r\n\x1b[48;2;55;55;55m\x1b[2K\x1b[0m\r\nBG_AUDIT_DONE\r\n")
