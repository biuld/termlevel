#!/usr/bin/env python3
import os,sys,select,termios,tty,time
fd=0
saved=termios.tcgetattr(fd)
try:
    tty.setraw(fd)
    os.write(1,b'\x1b]52;c;?\x07')
    received=b''
    deadline=time.monotonic()+1
    while time.monotonic()<deadline:
        if select.select([fd],[],[],.05)[0]:received+=os.read(fd,4096)
        if b'\x07' in received or b'\x1b\\' in received:break
    allowed=b'VEVSTUxFVkVMX09TQzUy' in received
    good=allowed==(sys.argv[1]=='allow')
    os.write(1,b'\r\nOSC52_READ_'+(b'ALLOW' if allowed else b'DENY')+(b'_OK' if good else b'_BAD')+b'\r\n')
finally:termios.tcsetattr(fd,termios.TCSANOW,saved)
