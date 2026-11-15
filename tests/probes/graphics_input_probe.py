"""Real PTY input echo while streaming large synchronized Kitty frames."""
import base64
import codecs
import os
import random
import select
import shutil
import termios
import tty

saved = termios.tcgetattr(0)
decode = codecs.getincrementaldecoder("utf-8")()
# Incompressible RGB data exercises the transport without PNG decode cost.
payload = base64.b64encode(random.Random(42).randbytes(192 * 192 * 3))
chunks = [payload[i:i + 4096] for i in range(0, len(payload), 4096)]
text = ""


def write(data):
    while data:
        count = os.write(1, data)
        data = data[count:]


try:
    tty.setraw(0)
    write(b"\x1b[?1049h\x1b[?25h\x1b[2J")
    while True:
        if select.select([0], [], [], 0.02)[0]:
            incoming = os.read(0, 4096)
            if not incoming or b"\x03" in incoming:
                break
            text += decode.decode(incoming)
        cols, rows = shutil.get_terminal_size()
        write(b"\x1b[?2026h\x1b[?25l\x1b_Ga=d,d=I,i=42,q=2;\x1b\\")
        write(f"\x1b[{max(1, rows - 5)};{max(1, cols - 8)}H".encode())
        for index, chunk in enumerate(chunks):
            more = int(index + 1 < len(chunks))
            if index == 0:
                header = f"\x1b_Ga=T,f=24,s=192,v=192,c=8,r=5,i=42,q=2,C=1,m={more};"
            else:
                header = f"\x1b_Gm={more};"
            write(header.encode() + chunk + b"\x1b\\")
        write(("\x1b[HINPUT_GRAPHICS_READY\x1b[2;1HINPUT_ECHO_" + text + "\x1b[?25h\x1b[?2026l").encode())
finally:
    write(b"\x1b[?2026l\x1b_Ga=d,d=I,i=42,q=2;\x1b\\\x1b[?25h\x1b[?1049l")
    termios.tcsetattr(0, termios.TCSANOW, saved)
