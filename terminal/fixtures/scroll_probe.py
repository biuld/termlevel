#!/usr/bin/env python3
"""Deterministic mouse-report TUI for native scroll profiling; no network."""
import os, re, select, termios, time, tty

saved = termios.tcgetattr(0)
start = time.monotonic()
step = 0
pending = b""
try:
    tty.setraw(0)
    os.write(1, b"\x1b[?1049h\x1b[?1000h\x1b[?1006h\x1b[?25l")
    def draw():
        cols, rows = os.get_terminal_size(0)
        out = ["\x1b[?2026h\x1b[H"]
        for row in range(rows):
            # Colour changes and fallback glyphs resemble an agent transcript.
            text = f"SCROLL_FRAME_{step:06d} " if row == 0 else f"{step + row:06d} │ "
            text += "\x1b[38;2;120;170;220mlet value = \x1b[0m"
            text += "中文 response text │ " * max(1, (cols - 48) // 21)
            out.append(text + "\x1b[0m\x1b[K")
            if row + 1 < rows:
                out.append("\r\n")
        out.append("\x1b[?2026l")
        os.write(1, "".join(out).encode())
    draw()
    while time.monotonic() - start < 60:
        if not select.select([0], [], [], .05)[0]:
            continue
        data = os.read(0, 8192)
        if b"q" in data:
            break
        pending += data
        reports = list(re.finditer(rb"\x1b\[<(\d+);\d+;\d+[Mm]", pending))
        if reports:
            for report in reports:
                button = int(report[1]) & 127
                if button in (64,65):
                    step += 1
            pending = pending[reports[-1].end():]
            draw()
finally:
    os.write(1, b"\x1b[?2026l\x1b[?1000l\x1b[?1006l\x1b[?25h\x1b[?1049l\r\nSCROLL_PROBE_DONE\r\n")
    termios.tcsetattr(0, termios.TCSANOW, saved)
