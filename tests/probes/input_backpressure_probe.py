#!/usr/bin/env python3
"""Real PTY reply storm: write queries before reading any replies."""
import os
import select
import sys
import time
import tty

count = 20000
tty.setraw(sys.stdin.fileno())
os.write(sys.stdout.fileno(), b"\x1b[H")
queries = b"\x1b[6n" * count
while queries:
    queries = queries[os.write(sys.stdout.fileno(), queries):]
# The writer must survive a client which temporarily does not read stdin.
time.sleep(0.1)
expected = b"\x1b[1;1R" * count
received = bytearray()
deadline = time.monotonic() + 4
while len(received) < len(expected):
    remaining = deadline - time.monotonic()
    if remaining <= 0 or not select.select([sys.stdin], [], [], remaining)[0]:
        print(f"INPUT_REPLIES_FAILED received={len(received)} expected={len(expected)}", flush=True)
        sys.exit(1)
    received.extend(os.read(sys.stdin.fileno(), len(expected) - len(received)))
if received != expected:
    print("INPUT_REPLIES_FAILED order or content mismatch", flush=True)
    sys.exit(2)
print(f"INPUT_REPLIES_OK count={count}", flush=True)
