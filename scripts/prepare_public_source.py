#!/usr/bin/env python3
"""Export a reviewed tracked tree without Git history; never publish/init a repo."""
import argparse
import hashlib
import io
import json
import posixpath
import re
from pathlib import Path, PurePosixPath
import subprocess
import tarfile

# Historical reports/evidence may include local paths or derived workshop details.
EXCLUDE_PREFIXES = ('.agents/', '.codex/', '.claude/', 'docs/validation/', 'docs/research/')
EXCLUDE_NAMES = {'AGENTS.md', 'CLAUDE.md', 'STATUS.md', 'ROADMAP.md', 'DEVLOG.md', 'scripts/build_wiki_presets.py'}


def excluded(name):
    return (name.startswith(EXCLUDE_PREFIXES) or name in EXCLUDE_NAMES
            or PurePosixPath(name).name.startswith(('FS_COMPAT_', 'FS_V1_', 'FS_PUBLIC_')))


def export(ref, destination):
    root = Path(__file__).resolve().parents[1]
    commit = subprocess.check_output(['git', 'rev-parse', '--verify', ref + '^{commit}'], cwd=root).decode().strip()
    payload = subprocess.check_output(['git', 'archive', '--format=tar', commit], cwd=root)
    # No extractall: reject links, traversal, special files and private paths.
    selected = {}
    removed = []
    with tarfile.open(fileobj=io.BytesIO(payload)) as archive:
        for member in archive:
            name = member.name
            path = PurePosixPath(name)
            if path.is_absolute() or '..' in path.parts or '.git' in path.parts:
                raise ValueError('unsafe archive path')
            if member.isdir():
                continue
            if not member.isfile():
                raise ValueError('links and special files are not exportable')
            if excluded(name):
                removed.append(name)
                continue
            if name.startswith(('data/lab/', 'data/backups/')) or name.endswith('.private.toml') or name == 'data/auth.toml':
                raise ValueError('private path in committed tree')
            selected[name] = (archive.extractfile(member).read(), member.mode & 0o777)
    # Fail rather than silently scrub a developer's source.
    for name, (content, _) in selected.items():
        if b'\0' not in content and any(marker in content for marker in (b'/' + b'Users/', b'/' + b'home/', b'C:' + bytes([92]) + b'Users' + bytes([92]))):
            raise ValueError('local path disclosure requires review: ' + name)
    # Documentation links to intentionally excluded internal evidence become plain
    # labels, with original hashes recorded. Implementation files stay byte-exact.
    original_hashes = {n: hashlib.sha256(b).hexdigest() for n, (b, _) in selected.items()}
    transformed = []
    for name, (content, mode) in list(selected.items()):
        if not name.endswith('.md'):
            continue
        def redact_link(match):
            label, target = match.groups()
            if ':' in target or target.startswith('#'):
                return match.group(0)
            resolved = posixpath.normpath(posixpath.join(posixpath.dirname(name), target.split('#')[0]))
            return label + ' (internal document excluded)' if excluded(resolved) else match.group(0)
        updated = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', redact_link, content.decode()).encode()
        if updated != content:
            selected[name] = (updated, mode)
            transformed.append(name)
    destination.mkdir(parents=False, exist_ok=False)
    for name, (content, mode) in selected.items():
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        path.chmod(mode)
    (destination / 'SOURCE_EXPORT.json').write_text(json.dumps({
        'source_commit': commit,
        'history_included': False,
        'publication_approved': False,
        'excluded': removed,
        'documentation_links_redacted': transformed,
        'source_sha256': original_hashes,
        'sha256': {n: hashlib.sha256(b).hexdigest() for n, (b, _) in selected.items()},
    }, indent=2) + '\n')
    print(f'Prepared {len(selected)} files; {len(removed)} excluded. No Git initialization or publication.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ref', required=True, help='Reviewed commit (not working-tree files)')
    parser.add_argument('destination', type=Path, help='New directory; parent must exist')
    args = parser.parse_args()
    export(args.ref, args.destination)
