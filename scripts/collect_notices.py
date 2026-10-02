#!/usr/bin/env python3
"""Collect Rust dependency notices on the build host; never include build-host paths."""
from pathlib import Path
import hashlib, json, subprocess, sys
root = Path(__file__).resolve().parents[1]
destination = Path(sys.argv[1]); destination.mkdir(parents=True, exist_ok=True)
packages = {}
for manifest, target in [("NativeHelper/Cargo.toml", "aarch64-apple-darwin"), ("Compatibility/Runtime/Cargo.toml", "x86_64-pc-windows-gnu")]:
    result = subprocess.check_output(["cargo", "metadata", "--locked", "--offline", "--format-version", "1", "--manifest-path", str(root / manifest), "--filter-platform", target], text=True)
    data = json.loads(result); active = {node["id"] for node in data["resolve"]["nodes"]}
    for package in data["packages"]:
        if package["id"] in active: packages[package["id"]] = package
index = ["# Rust dependency notices", "", "Generated from locked macOS-helper and Win64-runtime dependency graphs. License declarations are upstream metadata; original notice texts are retained alongside this index.", "", "| Package | Declared license | Upstream |", "|---|---|---|"]
texts = []; seen = set()
for package in sorted(packages.values(), key=lambda item: (item["name"], item["version"])):
    label = package["name"] + " " + package["version"]
    license_name = package.get("license") or "See source notices"
    repository = package.get("repository") or ""
    index.append("| " + label + " | " + license_name.replace("|", "/") + " | " + repository.replace("|", "%7C") + " |")
    base = Path(package["manifest_path"]).parent
    candidates = []
    for directory in [base, base.parent, base.parent.parent]:
        candidates = [file for file in directory.iterdir() if file.is_file() and file.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT"))]
        if candidates: break
    for file in sorted(candidates):
        content = file.read_text(errors="replace")
        digest = hashlib.sha256(content.encode()).hexdigest()
        if digest in seen: continue
        seen.add(digest); texts.append("\n## " + label + " / " + file.name + "\n\n" + content)
(destination / "Rust-Dependencies.md").write_text("\n".join(index) + "\n")
(destination / "Rust-License-Texts.txt").write_text("\n".join(texts))
print("Collected dependency declarations and available original notice texts.")
