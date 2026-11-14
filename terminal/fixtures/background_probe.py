#!/usr/bin/env python3
"""Mixed wide/ASCII runs with continuous terminal backgrounds for native QA."""
import os

cols, _ = os.get_terminal_size(0)
out = ["\x1b[2J\x1b[HBACKGROUND_READY\r\n\r\n"]
for text, background in [
    ("检查终端卡顿现状", "96;166;249"),
    ("1m ago    main", "96;166;249"),
    ("审查 terminal-grid 设计稿", "44;44;44"),
    ("核查 Terminal 功能完整性", "44;44;44"),
]:
    width = sum(2 if ord(c) > 127 else 1 for c in text)
    out.append(f"\x1b[48;2;{background}m\x1b[38;2;30;35;45m")
    # Force style boundaries without changing the background color.
    out.extend(("\x1b[1m" if n % 2 else "\x1b[22m") + c for n, c in enumerate(text))
    out.append(" " * max(0, cols - width) + "\x1b[0m\r\n")
os.write(1, "".join(out).encode())
