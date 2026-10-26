#!/usr/bin/env python3
"""Check extracted base-data against the reviewed release manifest (no Git needed)."""
import argparse
import hashlib
from pathlib import Path


def check(base):
    manifest = Path(__file__).resolve().parents[1] / 'crates/fabspeed-desktop/public-data.sha256'
    expected = {}
    for line in manifest.read_text().splitlines():
        digest, name = line.split('  ', 1)
        expected[name.removeprefix('data/')] = digest
    actual = {}
    for path in base.rglob('*'):
        if path.is_symlink():
            raise ValueError('symbolic resource link refused')
        if path.is_file():
            actual[path.relative_to(base).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise ValueError('resource inventory/hash mismatch (contents withheld)')
    print(f'PASS: exactly {len(expected)} approved public resources, byte-identical.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('base_data', type=Path)
    check(parser.parse_args().base_data)
