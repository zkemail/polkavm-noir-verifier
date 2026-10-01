#!/usr/bin/env python3
"""Regression test: regex matches must come from DKIM-signed bytes.

Runs `nargo execute` on a circuit with its sample inputs (must succeed), then on
inputs that copy a regex match into BoundedVec storage past `len()` (must fail).

NOTE: bytes of `header`, `body` and `decoded_body` at index >= len() are not
covered by the DKIM signature or the body hash, so a circuit that lets a regex
read them would output values that are not in the signed email.

Usage (needs nargo 1.0.0-beta.5 on PATH):
    python3 fixtures/zkemail/check_signed_bounds.py zkemail
    python3 check_signed_bounds.py <circuit_dir> <prover_toml> <body_regex_name>
"""

import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PRESETS = {
    "zkemail": (HERE, "Prover.toml", "x_handle"),
}


def section(text, name):
    m = re.search(r"(\[%s\]\s*\n)(.*?)(?=\n\[|\Z)" % re.escape(name), text, re.S)
    assert m, name
    return m


def get_list(body, key):
    raw = re.search(r"(?m)^%s\s*=\s*\[([^\]]*)\]" % key, body).group(1)
    return [int(x.strip().strip('"'), 0) for x in raw.split(",") if x.strip()]


def get_int(text, key):
    return int(re.search(r'(?m)^%s\s*=\s*"?(\d+)"?' % key, text).group(1))


def set_list(body, key, values):
    return re.sub(
        r"(?m)^(%s\s*=\s*)\[[^\]]*\]" % key,
        lambda m: m.group(1) + "[" + ", ".join('"%d"' % v for v in values) + "]",
        body,
        count=1,
    )


def set_int(text, key, value):
    return re.sub(r'(?m)^(%s\s*=\s*)"?\d+"?' % key, lambda m: m.group(1) + '"%d"' % value, text, count=1)


def write_tail(text, name, payload, at=None):
    """Write payload into `name`'s storage at `at` (default: len), leaving len unchanged."""
    m = section(text, name)
    storage = get_list(m.group(2), "storage")
    start = get_int(m.group(2), "len") if at is None else at
    storage[start : start + len(payload)] = list(payload)
    body = set_list(m.group(2), "storage", storage)
    return text[: m.start(2)] + body + text[m.end(2) :], start


def body_tail_case(text, regex):
    # Copy the signed body match (same NFA states) into the unsigned tail of both body arrays,
    # then point the body regex at the copy in decoded_body.
    m = section(text, "decoded_body")
    storage = get_list(m.group(2), "storage")
    start, length = get_int(text, regex + "_match_start"), get_int(text, regex + "_match_length")
    payload = bytes(storage[start : start + length])
    text, _ = write_tail(text, "body", payload)
    text, tail = write_tail(text, "decoded_body", payload)
    return set_int(text, regex + "_match_start", tail)


def header_tail_case(text, _regex):
    m = section(text, "header")
    storage = get_list(m.group(2), "storage")
    start, length = get_int(text, "sender_domain_match_start"), get_int(text, "sender_domain_match_length")
    text, tail = write_tail(text, "header", bytes(storage[start : start + length]))
    return set_int(text, "sender_domain_match_start", tail)


def decoded_len_case(text, regex):
    # A decoded_body longer than the body it was decoded from is never valid.
    m = section(text, "decoded_body")
    body_len = get_int(section(text, "body").group(2), "len")
    body = re.sub(r'(?m)^(len\s*=\s*)"?\d+"?', lambda mm: mm.group(1) + '"%d"' % (body_len + 1), m.group(2))
    return text[: m.start(2)] + body + text[m.end(2) :]


def run(circuit_dir, prover_text, label):
    with tempfile.TemporaryDirectory() as tmp:
        work = os.path.join(tmp, "circuit")
        shutil.copytree(circuit_dir, work, ignore=shutil.ignore_patterns("target", "files", "*.py"))
        with open(os.path.join(work, "Prover.toml"), "w") as f:
            f.write(prover_text)
        res = subprocess.run(["nargo", "execute"], cwd=work, capture_output=True, text=True)
        print("[%s] nargo execute -> exit %d" % (label, res.returncode))
        return res.returncode == 0, res.stdout + res.stderr


def main():
    if len(sys.argv) == 2:
        circuit_dir, prover, regex = PRESETS[sys.argv[1]]
    elif len(sys.argv) == 4:
        circuit_dir, prover, regex = sys.argv[1], sys.argv[2], sys.argv[3]
    else:
        sys.exit(__doc__)
    with open(os.path.join(circuit_dir, prover)) as f:
        base = f.read()

    ok, out = run(circuit_dir, base, "valid")
    if not ok:
        sys.exit("valid inputs failed:\n" + out)

    failures = []
    for label, mutate, expected in (
        ("body-tail", body_tail_case, "BoundedVec storage past len() must be zero"),
        ("header-tail", header_tail_case, "Header regex match must lie within the signed header"),
        ("decoded-len", decoded_len_case, "Decoded body cannot be longer than the encoded body"),
    ):
        ok, out = run(circuit_dir, mutate(base, regex), label)
        if ok or expected not in out:
            failures.append("%s: expected failure '%s'\n%s" % (label, expected, out[-2000:]))
    if failures:
        sys.exit("\n".join(failures))
    print("all signed-bounds checks passed")


if __name__ == "__main__":
    main()
