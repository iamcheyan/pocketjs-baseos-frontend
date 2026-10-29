#!/usr/bin/env python3
"""Convert a captured 720x480 (or 720x960 virtual) 32-bit fbdev frame to PNG."""
from pathlib import Path
import sys

from PIL import Image

if len(sys.argv) != 3:
    raise SystemExit(f"usage: {sys.argv[0]} INPUT.raw OUTPUT.png")
source, destination = map(Path, sys.argv[1:])
data = source.read_bytes()
visible = 720 * 480 * 4
virtual = 720 * 960 * 4
if len(data) == visible:
    image = Image.frombytes("RGBA", (720, 480), data, "raw", "BGRA")
elif len(data) == virtual:
    image = Image.frombytes("RGBA", (720, 960), data, "raw", "BGRA").crop((0, 0, 720, 480))
else:
    raise SystemExit(f"expected {visible} or {virtual} framebuffer bytes, got {len(data)}")
image.save(destination)
