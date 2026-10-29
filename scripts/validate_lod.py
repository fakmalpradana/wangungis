"""Validate CityJSON fixtures with cjval (schema) and val3dity (geometry).

tests/fixtures/lod/valid/*.city.json   must pass both validators.
tests/fixtures/lod/invalid/*.city.json must pass cjval and fail val3dity.

Both tools exit 0 whatever the verdict, so this reads their JSON reports.
Usage: python scripts/validate_lod.py [fixtures_dir]
"""

import json
import pathlib
import subprocess
import sys
import tempfile


def cjval_valid(path):
    out = subprocess.run(["cjval", "--report", str(path)], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)["valid"]


def val3dity_valid(path):
    with tempfile.TemporaryDirectory() as tmp:
        report = pathlib.Path(tmp) / "report.json"
        subprocess.run(["val3dity", "--report", str(report), str(path)], capture_output=True, check=True)
        return json.loads(report.read_text())["validity"]


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/lod")
    cases = [(p, True) for p in sorted((root / "valid").glob("*.city.json"))]
    cases += [(p, False) for p in sorted((root / "invalid").glob("*.city.json"))]
    if not cases:
        sys.exit(f"no fixtures under {root}")

    failures = 0
    for path, expect_geometry_valid in cases:
        schema = cjval_valid(path)
        geometry = val3dity_valid(path)
        ok = schema and geometry == expect_geometry_valid
        failures += not ok
        print(f"{'PASS' if ok else 'FAIL'} {path.name}: cjval={schema} val3dity={geometry} "
              f"(expected val3dity={expect_geometry_valid})")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
