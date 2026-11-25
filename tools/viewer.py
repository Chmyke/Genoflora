"""Builds a self-contained HTML viewer from the CSV snapshots written by the `germinate` probe.

Usage: python tools/viewer.py <snapshot dir> <out.html> [surface=2.0]

Standard library only. The page shows the plant cell by cell, coloured by one quantity at a time
(auxin, cytokinin, youth, light received, pumps...), whole or as a slice, over time.
"""

import json
import os
import sys

# Columns of the CSV, see crates/geno_gpu/examples/germinate.rs.
X, Y, Z, R, AUXIN, CYTOKININ, RESTRAINT, RELAY = 0, 1, 2, 3, 4, 5, 6, 7
PIN, YOUTH, LIGHT = 17, 20, 21
# Numbers kept per cell, see N in viewer.html.
KEPT = 13


def load(path):
    """One snapshot as a flat list: x, y, z, r, auxin and cytokinin concentrations, youth, light,
    pump polarity xyz, restraint signal and relay concentrations, per cell."""
    flat = []
    with open(path, encoding="ascii") as lines:
        for line in lines:
            v = [float(c) for c in line.split(",")]
            volume = v[R] ** 3
            cell = [v[X], v[Y], v[Z], v[R], v[AUXIN] / volume, v[CYTOKININ] / volume,
                    v[YOUTH], v[LIGHT], v[PIN], v[PIN + 1], v[PIN + 2], v[RESTRAINT] / volume,
                    v[RELAY] / volume]
            flat.extend(round(c, 3) for c in cell)
    return flat


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    source, out = sys.argv[1], sys.argv[2]
    surface = 2.0
    for arg in sys.argv[3:]:
        key, _, value = arg.partition("=")
        if key == "surface":
            surface = float(value)
    names = sorted(n for n in os.listdir(source) if n.endswith(".csv"))
    shots = [load(os.path.join(source, n)) for n in names]
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "viewer.html"), encoding="utf-8") as page:
        html = page.read()
    data = json.dumps({"surface": surface, "shots": shots}, separators=(",", ":"))
    with open(out, "w", encoding="utf-8", newline="\n") as target:
        target.write(html.replace("/*DATA*/null", data))
    print(f"{out}: {len(shots)} snapshots, {len(shots[-1]) // KEPT} cells in the last one")


if __name__ == "__main__":
    main()
