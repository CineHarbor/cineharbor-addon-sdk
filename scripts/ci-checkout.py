#!/usr/bin/env python3
"""Materialize the declared, immutable sibling dependency for a clean CI checkout."""
import json
import os
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[1]
spec = json.loads((root / "ci/dependency.json").read_text())
name, revision = spec["repository"], spec["revision"]
if name not in {"cineharbor-core", "cineharbor-addon-sdk"} or not re.fullmatch(r"[0-9a-f]{40}", revision):
    raise SystemExit("Invalid pinned dependency")
destination = root.parent / name
if destination.exists():
    raise SystemExit(f"Refusing to overwrite existing dependency: {destination}")
subprocess.run(["git", "init", str(destination)], check=True)
subprocess.run(["git", "-C", str(destination), "remote", "add", "origin", f"https://github.com/CineHarbor/{name}.git"], check=True)
subprocess.run(["git", "-C", str(destination), "fetch", "--depth", "1", "origin", revision], check=True)
subprocess.run(["git", "-C", str(destination), "checkout", "--detach", "FETCH_HEAD"], check=True)
actual = subprocess.check_output(["git", "-C", str(destination), "rev-parse", "HEAD"], text=True).strip()
if actual != revision:
    raise SystemExit("Dependency revision mismatch")
print(f"Dependency: CineHarbor/{name}@{actual}")
if os.environ.get("GITHUB_STEP_SUMMARY"):
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
        summary.write(f"\nDependency: `CineHarbor/{name}@{actual}`\n")
