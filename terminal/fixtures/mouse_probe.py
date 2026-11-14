#!/usr/bin/env python3
import os,select,termios,tty,time,re
saved=termios.tcgetattr(0)
try:
    tty.setraw(0)
    os.write(1,b'\x1b[?1000h\x1b[?1006h\r\nMOUSE_READY\r\n')
    received=b''
    deadline=time.monotonic()+5
    while time.monotonic()<deadline:
        if select.select([0],[],[],.05)[0]:received+=os.read(0,4096)
        if re.search(rb'\x1b\[<0;\d+;\d+m',received):break
    presses=re.findall(rb'\x1b\[<0;(\d+);(\d+)([Mm])',received)
    good=len(presses)==2 and presses[0][:2]==presses[1][:2] and presses[0][2]==b'M' and presses[1][2]==b'm' and all(int(n)>0 for n in presses[0][:2])
    os.write(1,b'\x1b[?1000l\x1b[?1006l\r\nMOUSE_EVENTS_'+(b'OK' if good else b'BAD')+b'\r\n')
finally:termios.tcsetattr(0,termios.TCSANOW,saved)
