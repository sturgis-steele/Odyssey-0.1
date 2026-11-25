"""Inventory locked crates and optionally collect their packaged license and notice files."""
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "backend-rust"


def cargo(*args):
    return subprocess.check_output(["cargo", *args], cwd=CRATE, encoding="utf-8")


def graph(target):
    tree = cargo("tree", "--locked", "--offline", "--target", target, "--edges", "normal,build",
                 "--prefix", "none", "--format", "{p}")
    return set(re.findall(r"^(\S+) v([0-9][^\s]*)", tree, flags=re.MULTILINE))


def notices(package):
    root = Path(package["manifest_path"]).parent
    chosen = set()
    if package.get("license_file"):
        chosen.add(Path(package["license_file"]))
    for item in root.iterdir():
        if item.name.lower().startswith(("license", "licence", "copying", "copyright", "notice", "unlicense")):
            if item.is_file():
                chosen.add(item)
            elif item.is_dir():
                chosen.update(p for p in item.rglob("*") if p.is_file())
    return sorted(chosen)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--notices", type=Path, help="Write a combined notice candidate for packaging review")
    args = p.parse_args()
    meta = json.loads(cargo("metadata", "--locked", "--offline", "--format-version", "1"))
    packages = [p for p in meta["packages"] if p["source"]]
    app, = (p for p in meta["packages"] if p["name"] == "odyssey-backend")
    direct = {d["name"]: d["kind"] or "direct" for d in app["dependencies"]}
    win = graph("x86_64-pc-windows-gnu")
    arm = graph("aarch64-unknown-linux-gnu")
    rows = ["# Rust dependency inventory", "",
            f"Cargo.lock resolves {len(packages)} external crate versions. The table includes development, build and target-specific packages. Windows and ARM64 columns include normal and build-time dependencies. Other entries belong to tests or additional target resolutions.", "",
            "The license column reproduces the packaged manifest expression. A slash is an older dual-license spelling. The file count covers packaged top-level license, copyright and notice files and license directories. Review embedded source and data notices before shipping. See [distribution considerations](distribution-considerations.md).", "",
            "Regenerate with `python scripts/audit-rust-licenses.py` after `cargo fetch --locked`. Add `--notices PATH` to collect upstream text for manual review. The inventory and output include build-only dependencies as a conservative superset.", "",
            "| Crate and version | Relationship | Windows | ARM64 | Declared license | Notice files |",
            "| --- | --- | --- | --- | --- | ---: |"]
    output = []
    for package in sorted(packages, key=lambda v: (v["name"], v["version"])):
        n, v = key = package["name"], package["version"]
        files = notices(package)
        rows.append(f"| [{n} {v}](https://crates.io/crates/{n}/{v}) | {direct.get(n, 'transitive')} | "
                    f"{'yes' if key in win else '–'} | {'yes' if key in arm else '–'} | {package['license'] or 'review license_file'} | {len(files)} |")
        if args.notices and (key in arm or key in win):
            output.append(f"\n## {n} {v}\n\n{package['license']}\n")
            if not files:
                output.append("No standalone license file found. Inspect the published crate before packaging.\n")
            for file in files:
                output.extend([file.name + "\n", file.read_text(encoding="utf-8", errors="replace"), "\n"])
    target = ROOT / "docs/licensing/rust-dependencies.md"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("\n".join(rows) + "\n", encoding="utf-8")
    if args.notices:
        args.notices.write_text("\n".join(output), encoding="utf-8")


if __name__ == "__main__":
    main()
