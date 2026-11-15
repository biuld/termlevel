#!/usr/bin/env python3
"""Real PTY peer for native protocol acceptance; never used by production."""
import os, select, sys, termios, tty, time
fd = sys.stdin.fileno()
saved = termios.tcgetattr(fd)
try:
    tty.setraw(fd)
    os.write(1,b'\x1b[>31u\r\nKITTY_READY\r\n')
    received=b''
    deadline=time.monotonic()+3
    while received.count(b'u') < 6 and time.monotonic()<deadline:
        if select.select([fd],[],[],.1)[0]:
            received+=os.read(fd,4096)
    expected=b'\x1b[97;;97u\x1b[97;1:2;97u\x1b[97;1:3u\x1b[13u\x1b[13;1:2u\x1b[13;1:3u'
    os.write(1,b'\x1b[<u\r\n'+(b'KITTY_EVENTS_OK' if received==expected else b'KITTY_EVENTS_BAD_'+received.hex().encode())+b'\r\n')
finally:
    termios.tcsetattr(fd,termios.TCSANOW,saved)
