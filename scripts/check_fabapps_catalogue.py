#!/usr/bin/env python3
"""Independent test/reference reader. Requires jsonschema==4.23.0.
No cache, network, publisher or FabCAM dependency. Never executes resource text.
"""
import copy
import datetime
import hashlib
import json
import math
from pathlib import Path
import struct
import sys
from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1] / "docs/fabapps/catalogue/v1"

def canonical(value):
    if value is None:
        return b"n"
    if isinstance(value, bool):
        return b"t" if value else b"f"
    if isinstance(value, (int, float)):
        if not math.isfinite(value) or (isinstance(value, int) and abs(value) > 9007199254740991):
            raise ValueError("invalid interoperable number")
        return b"d" + struct.pack(">d", 0.0 if value == 0 else float(value))
    if isinstance(value, str):
        raw = value.encode("utf-8")
        return b"s" + len(raw).to_bytes(8, "big") + raw
    if isinstance(value, list):
        return b"a" + len(value).to_bytes(8, "big") + b"".join(map(canonical, value))
    return b"o" + len(value).to_bytes(8, "big") + b"".join(
        canonical(key) + canonical(value[key]) for key in sorted(value, key=lambda k: k.encode("utf-8")))

def digest(domain, value):
    return "sha256:" + hashlib.sha256(b"fabapps-c14n-1\0" + domain.encode() + b"\0" + canonical(value)).hexdigest()

def without(value, *keys):
    return {k: v for k, v in value.items() if k not in keys}

def validate(value):
    schema = json.loads((ROOT / "fabapps-catalogue-v1.schema.json").read_text())
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema, format_checker=FormatChecker()).validate(value)
    dates = [value["published_at"]]
    for preset in value["presets"]:
        dates.extend([preset["validation"]["tested_at"], preset["validation"]["validated_at"]])
    for text in dates:
        if not text.endswith("Z"):
            raise ValueError("timestamp must be UTC")
        datetime.datetime.fromisoformat(text.replace("Z", "+00:00"))
    canonical(value)  # reject non-finite extensions accepted by some JSON parsers
    maps = {}
    for plural, domain in [("machines", "machine"), ("tools", "tool"), ("materials", "material"), ("presets", "preset")]:
        rows = value[plural]
        ids = [r["id"] for r in rows]
        if ids != sorted(set(ids)):
            raise ValueError("duplicate or unordered identity")
        maps[plural] = {r["id"]: r for r in rows}
        for row in rows:
            if digest(domain, without(row, "revision")) != row["revision"]:
                raise ValueError("resource revision mismatch")
    for row in value["machines"]:
        limits = row["limits"]
        if limits["rpm_min"]["value"] > limits["rpm_max"]["value"]:
            raise ValueError("inverted machine range")
    for t in value["tools"]:
        if "overall_length_mm" in t and any(t.get(k, 0) > t["overall_length_mm"] for k in ["flute_length_mm", "upcut_length_mm"]):
            raise ValueError("inconsistent tool lengths")
        if t.get("upcut_length_mm", 0) > t["flute_length_mm"]:
            raise ValueError("inconsistent upcut length")
        if "max_rpm" in t and any(t.get(k, 0) > t["max_rpm"] for k in ["supplier_rpm_min", "supplier_rpm_recommended"]):
            raise ValueError("inverted tool range")
        if "supplier_rpm_min" in t and "supplier_rpm_recommended" in t and t["supplier_rpm_min"] > t["supplier_rpm_recommended"]:
            raise ValueError("inconsistent supplier range")
    for p in value["presets"]:
        reference = p["validation"]["trial_id"][7:]
        if p["id"] != "trial:" + reference or p["name"] != "Validated preset " + reference[:12]:
            raise ValueError("inconsistent opaque trial reference")
        for kind, plural in [("machine", "machines"), ("tool", "tools"), ("material", "materials")]:
            ref = p[kind]
            if ref["id"] not in maps[plural] or maps[plural][ref["id"]]["revision"] != ref["revision"]:
                raise ValueError("broken dependency")
    if digest("catalogue", without(value, "revision", "generation", "published_at")) != value["revision"]:
        raise ValueError("catalogue revision mismatch")
    return value

def accept(previous, incoming):
    """Reference acceptance decision only; no production consumer cache."""
    validate(incoming)
    if incoming["catalogue_id"] != previous["catalogue_id"]:
        raise ValueError("different source")
    if incoming["generation"] < previous["generation"]:
        raise ValueError("older generation")
    if incoming["generation"] == previous["generation"] and incoming["revision"] != previous["revision"]:
        raise ValueError("equal generation with different content")
    return incoming["generation"] > previous["generation"]

def rehash(v):
    for plural, domain in [("machines", "machine"), ("tools", "tool"), ("materials", "material"), ("presets", "preset")]:
        for row in v[plural]:
            row["revision"] = digest(domain, without(row, "revision"))
    v["revision"] = digest("catalogue", without(v, "revision", "generation", "published_at"))
    return v

def self_test():
    vectors = json.loads((ROOT / "canonical-vectors.json").read_text())
    for vector in vectors:
        assert canonical(vector["value"]).hex() == vector["canonical_hex"]
        assert digest(vector["domain"], vector["value"]) == vector["revision"]
    assert digest("vector", {"a": 1, "z": 2}) == digest("vector", {"z": 2, "a": 1})
    good = validate(json.loads((ROOT / "fabapps-catalogue-v1.synthetic.json").read_text()))
    negatives = []
    def case(change):
        v = copy.deepcopy(good); change(v); negatives.append(v)
    case(lambda v: v.update(version=2))
    case(lambda v: v.pop("tools"))
    case(lambda v: v.update(complete=False))
    case(lambda v: v.update(generation=0))
    case(lambda v: v.update(published_at="2026-99-99T00:00:00Z"))
    case(lambda v: v["tools"][0].update(family="unknown_family"))
    case(lambda v: v["tools"].append(copy.deepcopy(v["tools"][0])))
    case(lambda v: v["presets"][0]["tool"].update(id="absent"))
    case(lambda v: v["presets"][0]["values"].update(feed_mm_min=-1))
    case(lambda v: v["presets"][0]["values"].update(feed_mm_min=float("nan")))
    case(lambda v: v["presets"][0]["values"].update(feed_unit="inch/min"))
    case(lambda v: v["presets"][0]["context"].update(stepover_ratio=1.1))
    case(lambda v: v["tools"][0].update(notes="PRIVATE"))
    case(lambda v: v["presets"][0].update(name="PRIVATE_OPERATOR"))
    case(lambda v: v["tools"][0].update(diameter_mm=3.125)) # stale hash
    for index, v in enumerate(negatives):
        try:
            validate(v)
        except Exception as error:
            # Schema errors and semantic errors both reject the whole snapshot.
            assert error is not None
        else:
            raise AssertionError(f"invalid fixture {index} accepted")
    try:
        validate(json.loads((ROOT / "fabapps-catalogue-v1.invalid.json").read_text()))
    except Exception:
        pass
    else:
        raise AssertionError("invalid checked-in fixture accepted")
    # Semantic faults remain rejected even if a faulty producer recomputes hashes.
    for change in [lambda v: v["presets"][0]["tool"].update(id="missing"), lambda v: v["machines"][0]["limits"]["rpm_min"].update(value=999999)]:
        v=copy.deepcopy(good); change(v);rehash(v)
        try: validate(v)
        except ValueError: pass
        else: raise AssertionError("semantic fault accepted")
    empty=copy.deepcopy(good)
    for k in ["machines","tools","materials","presets"]: empty[k]=[]
    rehash(empty)
    try: accept(good,empty)
    except ValueError: pass
    else: raise AssertionError("equal-generation replacement accepted")
    empty["generation"]=2
    assert accept(good,empty)  # a valid complete newer empty set withdraws availability
    try: accept(empty,good)
    except ValueError: pass
    else: raise AssertionError("rollback accepted")
    assert not accept(good,good)
    # No incoming valid envelope means no acceptance/withdrawal decision.
    for raw in ["", "{", '{"complete":true}']:
        try: validate(json.loads(raw))
        except Exception: pass
        else: raise AssertionError("partial snapshot accepted")
    print(f"PASS: schema, {len(vectors)} independent vectors, {len(negatives)} negative mutations, semantic references/ranges and generation rules")

if __name__ == "__main__":
    try:
        if len(sys.argv) == 2:
            validate(json.loads(Path(sys.argv[1]).read_text()))
            print("PASS: complete catalogue")
        else:
            self_test()
    except Exception:
        # Input, schema diagnostics and paths can contain private strings.
        print("Catalogue conformance rejected", file=sys.stderr)
        sys.exit(1)
