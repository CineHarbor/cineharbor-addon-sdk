#!/usr/bin/env python3
"""Format all owned workspace members, never sibling path dependencies."""
import argparse
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--check", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root, text=True))
members = set(metadata["workspace_members"])
packages = [package for package in metadata["packages"] if package["id"] in members]
if not packages:
    raise SystemExit("No workspace members; refusing an empty formatting gate")
command = ["cargo", "fmt"]
for package in sorted(packages, key=lambda package: package["name"]):
    if not Path(package["manifest_path"]).resolve().is_relative_to(root):
        raise SystemExit("Workspace ownership escaped repository")
    command.extend(["--package", package["name"]])
if args.check:
    command.extend(["--", "--check"])
print(" ".join(command), flush=True)
subprocess.run(command, cwd=root, check=True)
