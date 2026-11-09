"""Create the source companion to an RF-EPiano release."""
import argparse
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

ROOT = Path(__file__).resolve().parents[1]
FILES = (".gitignore", "Cargo.toml", "Cargo.lock", "LICENSE", "README.md", "rust-toolchain.toml",
         ".cargo/config.toml", ".cargo/config.local.toml.example")
DIRECTORIES = ("plugin", "package", "vendor", "tools", "docs")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    paths = [ROOT / name for name in FILES]
    for directory in DIRECTORIES:
        paths.extend(p for p in (ROOT / directory).rglob("*") if p.is_file()
                     and "__pycache__" not in p.parts and p.suffix not in {".wasm", ".pyc"})
    with ZipFile(args.output, "x", compression=ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(paths):
            archive.write(path, f"RF-EPiano-0.1.5/{path.relative_to(ROOT).as_posix()}")
    with ZipFile(args.output) as archive:
        if archive.testzip() is not None:
            raise RuntimeError("source archive verification failed")
    print(f"RF-EPIANO_SOURCE_OK path={args.output} files={len(paths)}")


if __name__ == "__main__":
    main()
