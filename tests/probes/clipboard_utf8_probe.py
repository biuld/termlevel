#!/usr/bin/env python3
"""OSC52 UTF-8 write/read through the host's actual system clipboard."""
import base64
import os
import select
import termios
import time
import tty

text = "Claude 中文复制：你好，世界！ café e\u0301 😀\n第二行"
payload = base64.b64encode(text.encode("utf-8"))
saved = termios.tcgetattr(0)
try:
    tty.setraw(0)
    os.write(1, b"\x1b]52;c;" + payload + b"\x07")
    os.write(1, b"\x1b]52;c;?\x07")
    received = b""
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        if select.select([0], [], [], 0.05)[0]:
            received += os.read(0, 8192)
        if b"\x07" in received or b"\x1b\\" in received:
            break
    good = received in (b"\x1b]52;c;" + payload + b"\x07", b"\x1b]52;c;" + payload + b"\x1b\\")
    os.write(1, b"\r\nOSC52_UTF8_" + (b"OK" if good else b"BAD") + b"\r\n")
finally:
    termios.tcsetattr(0, termios.TCSANOW, saved)
