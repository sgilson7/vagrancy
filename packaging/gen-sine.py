#!/usr/bin/env python3
"""Print the quarter-wave sine table that crates/sim/src/fx.rs carries.

The table is generated here, once, and its output is committed, because `sim`
may not compute a float and the table must be the same bytes on every
machine. Entry d is round(4096 * sin(d degrees)) for d in 0..=90. H6 pins
three of the values: 0 at 0 degrees, 4096 at 90 and exactly 2048 at 30.

    python3 packaging/gen-sine.py
"""
import math
vals = [round(4096 * math.sin(math.radians(d))) for d in range(91)]
assert vals[0] == 0 and vals[30] == 2048 and vals[90] == 4096
rows = [", ".join(str(v) for v in vals[i:i + 10]) for i in range(0, 91, 10)]
print("const SIN_DEG: [i32; 91] = [\n    " + ",\n    ".join(rows) + ",\n];")
