"""Generate the 1024x1024 source app icon (a float on water).

Usage: python3 -I tools/gen_icon.py OUT.png  (then `npx tauri icon OUT.png`)
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from png import write_png  # noqa: E402

S = 1024


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def main(out):
    px = bytearray(S * S * 4)
    cx, cy, r = S / 2, S / 2, S * 0.46
    for y in range(S):
        for x in range(S):
            dx, dy = x - cx, y - cy
            d = math.hypot(dx, dy)
            if d > r:
                continue
            a = 255 if d < r - 1 else int(255 * (r - d))
            # water gradient + ripples
            col = lerp((70, 130, 140), (22, 52, 60), y / S)
            ring = math.hypot(dx, (dy - S * 0.12) * 2.6)
            if y > S * 0.52 and abs((ring % 90) - 45) < 4:
                col = lerp(col, (180, 215, 220), 0.5)
            # float body: red top, white bottom, antenna
            fx, fy = dx, y - S * 0.40
            body = (fx / (S * 0.11)) ** 2 + (fy / (S * 0.20)) ** 2
            if body <= 1 and y < S * 0.60:
                col = (222, 70, 52) if fy < 0 else (240, 238, 228)
            if abs(dx) < S * 0.012 and S * 0.12 < y < S * 0.25:
                col = (222, 70, 52)
            i = (y * S + x) * 4
            px[i : i + 4] = bytes((*col, a))
    write_png(out, S, S, bytes(px))


if __name__ == "__main__":
    main(sys.argv[1])
