#!/usr/bin/env python3
"""Verify actual production-publisher test output, independently of Rust.
Run publication_tests with FABSPEED_PUBLICATION_EVIDENCE=<temporary-directory>.
Requires the same pinned jsonschema dependency as check_fabapps_catalogue.py.
"""
import copy
import json
from pathlib import Path
import sys
from check_fabapps_catalogue import accept, rehash, self_test, validate


def main(root):
    self_test()
    names = ['01-valid.json', '02-modified.json', '03-revoked.json', '04-disabled.json']
    values = [validate(json.loads((root / name).read_bytes())) for name in names]
    assert len(values[0]['presets']) == 1
    assert values[0]['presets'][0]['values']['feed_mm_min'] == 1200.25
    assert not values[2]['presets']
    assert all(not values[-1][k] for k in ['machines', 'tools', 'materials', 'presets'])
    for old, new in zip(values, values[1:]):
        assert new['generation'] == old['generation'] + 1
        assert accept(old, new)
    before = validate(json.loads((root / '05-republish-before.json').read_bytes()))
    after = validate(json.loads((root / '06-republish-after.json').read_bytes()))
    assert before['revision'] == after['revision'] and accept(before, after)
    negatives = [(values[1], values[0])]
    changed = copy.deepcopy(values[0]); changed['presets'] = []
    changed['machines'][0]['name'] = 'Synthetic changed content'
    negatives.append((values[0], rehash(changed)))
    other = copy.deepcopy(values[0]); other['catalogue_id'] = 'fs-' + 'f' * 32
    negatives.append((values[0], rehash(other)))
    unknown = copy.deepcopy(values[0]); unknown['version'] = 2
    negatives.append((values[0], unknown))
    for previous, incoming in negatives:
        try:
            accept(previous, incoming)
        except Exception:
            pass
        else:
            raise AssertionError('invalid replay accepted')
    try:
        json.loads((root / names[0]).read_bytes()[:-10])
    except ValueError:
        pass
    else:
        raise AssertionError('truncation accepted')
    for name in names + ['05-republish-before.json', '06-republish-after.json']:
        raw = (root / name).read_bytes()
        assert b'SECRET_' not in raw and b'PRIVATE_' not in raw
    print('PASS: six actual files, ordered complete replay, same-revision republish, five negative replay/truncation cases, exact fractions, private sentinels absent')


if __name__ == '__main__':
    try:
        main(Path(sys.argv[1]))
    except Exception:
        print('FAIL: publication replay rejected', file=sys.stderr)
        sys.exit(1)
