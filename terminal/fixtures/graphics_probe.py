#!/usr/bin/env python3
"""Small inline Kitty/OSC fixtures for the real native terminal surface."""
import base64,struct,zlib,os

def chunk(kind,data):
    return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
raw=b'\x00'+bytes([255,0,0,255,0,255,0,255])+b'\x00'+bytes([0,0,255,255,255,255,0,255])
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')
os.write(1,b'\x1b[2J\x1b[H\x1b]8;;https://example.com\x1b\\LINKED_WORD\x1b]8;;\x1b\\\r\n')
os.write(1,b'\x1b_Ga=T,f=100,q=2,i=77,c=16,r=6;'+base64.b64encode(png)+b'\x1b\\')
os.write(1,b'\r\nGRAPHICS_READY\r\n\x1b]12;#ff00ff\x07\x1b[6 q\x1b]52;c;VEVSTUxFVkVMX09TQzUy\x07')
