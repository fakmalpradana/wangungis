"""Validate CityJSON fixtures with cjval (schema) and val3dity (geometry).

tests/fixtures/lod/valid/*.city.json   must pass both validators.
tests/fixtures/lod/invalid/<code>-*.city.json must pass cjval and fail val3dity
with error <code> (e.g. 307-*.city.json expects POLYGON_WRONG_ORIENTATION).

Both tools exit 0 whatever the verdict, so this reads their JSON reports.
Usage: python scripts/validate_lod.py [fixtures_dir]
"""

import json
import pathlib
import subprocess
import sys
import tempfile


def cjval_clean(path):
    """Schema valid and no warnings (duplicate/unused vertices, extra root properties)."""
    out = subprocess.run(["cjval", "--report", str(path)], capture_output=True, text=True, check=True)
    report = json.loads(out.stdout)
    return report["valid"] and not report["has_warnings"]


def val3dity_report(path):
    with tempfile.TemporaryDirectory() as tmp:
        report = pathlib.Path(tmp) / "report.json"
        subprocess.run(["val3dity", "--report", str(report), str(path)], capture_output=True, check=True)
        return json.loads(report.read_text())


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/lod")
    valid = sorted((root / "valid").glob("*.city.json"))
    invalid = sorted((root / "invalid").glob("*.city.json"))
    if not valid or not invalid:
        sys.exit(f"need fixtures in both {root}/valid and {root}/invalid")

    failures = 0
    for path in valid + invalid:
        schema = cjval_clean(path)
        r = val3dity_report(path)
        checked = sum(p["total"] for p in r["primitives_overview"])
        # Geometry was actually validated: no file-level errors, at least one primitive.
        ok = schema and checked > 0 and not r["dataset_errors"]
        if path in valid:
            expected = "valid"
            ok = ok and r["validity"]
        else:
            code = int(path.name.split("-", 1)[0])
            expected = f"error {code}"
            ok = ok and not r["validity"] and code in r["all_errors"]
        failures += not ok
        print(f"{'PASS' if ok else 'FAIL'} {path.name}: cjval={schema} val3dity={r['validity']} "
              f"errors={r['all_errors']} primitives={checked} (expected {expected})")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
