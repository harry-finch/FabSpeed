#!/usr/bin/env python3
"""Build existing desktop formats while removing builder home/source paths."""
import argparse
import os
from pathlib import Path
import shlex
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundles', required=True)
    parser.add_argument('--target')
    parser.add_argument('--config', help='Optional supported Tauri JSON configuration override')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    env = os.environ.copy()
    # Encoded flags preserve spaces in Windows/user paths. Existing flags survive.
    encoded = env.get('CARGO_ENCODED_RUSTFLAGS')
    flags = encoded.split('\x1f') if encoded else shlex.split(env.get('RUSTFLAGS', ''))
    flags += [f'--remap-path-prefix={root}=/source',
              f'--remap-path-prefix={Path.home()}=/builder']
    env.pop('RUSTFLAGS', None)
    env['CARGO_ENCODED_RUSTFLAGS'] = '\x1f'.join(flags)
    npm = 'npm.cmd' if os.name == 'nt' else 'npm'
    command = [npm, 'exec', '--yes', '--package=@tauri-apps/cli@2.12.3', '--',
               'tauri', 'build', '--bundles', args.bundles]
    if args.target:
        command += ['--target', args.target]
    if args.config:
        command += ['--config', str(Path(args.config).resolve())]
    subprocess.run(command + ['--', '--locked'], cwd=root / 'crates/fabspeed-desktop',
                   env=env, check=True)


if __name__ == '__main__':
    main()
