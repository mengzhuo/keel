#!/usr/bin/env python3
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

"""Compare crabc's preprocessor macro expansions with musl 1.2.6.

The comparison is deliberately done after expansion, not on the #define text:
musl and crabc may spell the same constant differently (0x100 vs 256), while a
real incompatibility is a different token sequence after preprocessing.

For each header that both trees provide (internal bits/ headers are skipped),
every macro defined by both sides is expanded in isolation and the resulting
token sequence is normalized (whitespace, integer spelling, redundant outer
parentheses).  Function-like macros are invoked with the same dummy arguments
on both sides.  Differences must be covered by an explicit allowlist entry.

Missing macros are reported as coverage statistics but do not fail this check;
the check is about the expansion of the ABI surface that crabc actually ships.
"""

import argparse
import fnmatch
import os
import re
import subprocess
import sys

LANGUAGE = "c"

TOKEN_RE = re.compile(
    r'[A-Za-z_]\w*'
    r'|0[xX][0-9a-fA-F]+[uUlL]*'
    r'|(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?[fFlL]?'
    r'|"(?:\\.|[^"\\])*"'
    r"|'(?:\\.|[^'\\])*'"
    r'|<<|>>|<=|>=|==|!=|&&|\|\||->|\+\+|--|\S'
)
INT_RE = re.compile(r'^(0[xX][0-9a-fA-F]+|0[0-7]*|[1-9][0-9]*)([uUlL]*)$')


def norm_int(token):
    match = INT_RE.match(token)
    if not match:
        return token
    digits, suffix = match.groups()
    try:
        if digits.lower().startswith("0x"):
            value = int(digits, 16)
        elif len(digits) > 1 and digits.startswith("0"):
            value = int(digits, 8)
        else:
            value = int(digits, 10)
    except ValueError:
        return token
    # All supported targets are LP64, where L and LL have the same width.
    return f"{value}{'U' if 'u' in suffix.lower() else ''}"


def strip_outer(tokens):
    while len(tokens) >= 2 and tokens[0] == "(" and tokens[-1] == ")":
        depth = 0
        for index, token in enumerate(tokens):
            if token == "(":
                depth += 1
            elif token == ")":
                depth -= 1
                if depth == 0 and index != len(tokens) - 1:
                    return tokens
        if depth != 0:
            return tokens
        tokens = tokens[1:-1]
    return tokens


def normalize(text):
    tokens = [norm_int(token) for token in TOKEN_RE.findall(text)]
    return " ".join(strip_outer(tokens))


def run_cc(cc, include, args, source=None):
    if LANGUAGE == "c++":
        standard, dialect = "-std=gnu++17", "c++"
    else:
        standard, dialect = "-std=gnu11", "c"
    command = [
        cc,
        standard,
        "-D_GNU_SOURCE",
        "-nostdinc",
        "-I",
        include,
        *args,
        "-x",
        dialect,
        "-",
    ]
    return subprocess.run(
        command,
        input=source if source is not None else "",
        capture_output=True,
        text=True,
        timeout=120,
    )


def macro_table(cc, include, header):
    proc = run_cc(cc, include, ["-dM", "-E", "-include", header])
    if proc.returncode != 0:
        detail = proc.stderr.strip().splitlines()
        return None, detail[-1] if detail else "preprocessor failed"
    table = {}
    for line in proc.stdout.splitlines():
        if not line.startswith("#define "):
            continue
        match = re.match(
            r"([A-Za-z_]\w*)(?:\(([^)]*)\))?(?:\s+(.*))?$", line[len("#define "):]
        )
        if not match:
            continue
        name, params, body = match.groups()
        if params is None:
            kind, parameters = "object", []
        else:
            kind = "function"
            parameters = [
                item.strip() for item in params.split(",") if item.strip()
            ]
        table[name] = {
            "kind": kind,
            "params": parameters,
            "variadic": "..." in parameters,
            "body": body or "",
        }
    return table, None


def macro_shape(macro):
    return (
        macro["kind"],
        macro["variadic"],
        len([item for item in macro["params"] if item != "..."]),
    )


def probe_source(header, macros):
    lines = ["#define _GNU_SOURCE 1", f"#include <{header}>"]
    markers = []
    for index, (name, macro) in enumerate(macros):
        marker = f"int __crabc_probe_marker_{index};"
        lines.append('#line 1000 "probe.c"')
        lines.append(marker)
        if macro["kind"] == "object":
            lines.append(name)
        else:
            fixed = [item for item in macro["params"] if item != "..."]
            args = [f"a{position + 1}" for position in range(len(fixed))]
            if macro["variadic"]:
                args += ["a1", "a2", "a3"]
            lines.append(f"{name}({', '.join(args)})")
        markers.append(marker)
    end_marker = "int __crabc_probe_marker_end;"
    lines.append(end_marker)
    markers.append(end_marker)
    return "\n".join(lines) + "\n", markers


def split_expansions(output, markers):
    lines = output.splitlines()
    positions = []
    position = 0
    for marker in markers:
        found = -1
        for index in range(position, len(lines)):
            if lines[index].strip() == marker:
                found = index
                break
        if found < 0:
            return None
        positions.append(found)
        position = found + 1
    expansions = []
    for index in range(len(markers) - 1):
        text = " ".join(
            line.strip() for line in lines[positions[index] + 1:positions[index + 1]]
        )
        expansions.append(normalize(text))
    return expansions


def expand_macros(cc, include, header, entries):
    source, markers = probe_source(header, entries)
    proc = run_cc(cc, include, ["-E", "-P"], source)
    if proc.returncode != 0:
        return None
    return split_expansions(proc.stdout, markers)


def load_allowlist(path):
    entries = []
    if not path or not os.path.exists(path):
        return entries
    with open(path, encoding="utf-8") as handle:
        for raw in handle:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split(None, 3)
            if len(parts) < 3:
                raise SystemExit(f"invalid allowlist line: {raw.rstrip()}")
            entries.append(
                (
                    parts[0],
                    parts[1],
                    parts[2],
                    parts[3] if len(parts) > 3 else "",
                )
            )
    return entries


def allowlist_match(entries, arch, header, macro):
    for entry_arch, entry_header, entry_macro, reason in entries:
        if (
            fnmatch.fnmatchcase(arch, entry_arch)
            and fnmatch.fnmatchcase(header, entry_header)
            and fnmatch.fnmatchcase(macro, entry_macro)
        ):
            return reason
    return None


def is_abi_macro(name):
    """Macros that are part of musl's usable surface.

    Include guards, the __NEED_/__DEFINED_ type plumbing, feature-test
    switches, and Linux UAPI include guards are implementation details rather
    than macros a program can use.
    """
    if re.match(r"^_[A-Z0-9_]*_H$", name):
        return False
    if name.startswith(("__NEED_", "__DEFINED_", "__USE_", "__REDIR", "__UAPI_DEF_")):
        return False
    if name in ("__PIC__", "__PIE__"):
        return False
    return True


def load_coverage_baseline(path):
    baseline = {}
    if not path or not os.path.exists(path):
        return baseline
    with open(path, encoding="utf-8") as handle:
        for raw in handle:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) >= 2:
                baseline[parts[0]] = (int(parts[1]), int(parts[2]) if len(parts) > 2 else 0)
    return baseline


def compare(args):
    allowlist = load_allowlist(args.allowlist)
    coverage_baseline = load_coverage_baseline(args.coverage_baseline)
    headers = []
    for root, _, files in os.walk(args.musl):
        for name in files:
            relative = os.path.relpath(os.path.join(root, name), args.musl)
            if relative.startswith("bits/"):
                continue
            if os.path.exists(os.path.join(args.crabc, relative)):
                headers.append(relative)
    headers.sort()
    missing_headers = sorted(
        header
        for header in headers
        if not os.path.exists(os.path.join(args.crabc, header))
    )

    mismatches = []
    failures = []
    skipped = []
    allowlisted = 0
    used_allowlist = set()
    musl_abi = set()
    crabc_abi = set()
    header_missing = {}
    common_count = 0

    for header in headers:
        musl_table, musl_error = macro_table(args.cc, args.musl, header)
        if musl_table is None:
            skipped.append((header, f"musl: {musl_error}"))
            continue
        crabc_table, crabc_error = macro_table(args.cc, args.crabc, header)
        if crabc_table is None:
            failures.append((header, f"crabc: {crabc_error}"))
            continue
        musl_here = {name for name in musl_table if is_abi_macro(name)}
        crabc_here = {name for name in crabc_table if is_abi_macro(name)}
        musl_abi.update(musl_here)
        crabc_abi.update(crabc_here)
        absent = sorted(musl_here - crabc_here)
        if absent:
            header_missing[header] = absent

        common = sorted(set(musl_table) & set(crabc_table))
        common_count += len(common)
        probes = []
        for name in common:
            if "__COUNTER__" in musl_table[name]["body"]:
                continue
            if macro_shape(musl_table[name]) != macro_shape(crabc_table[name]):
                mismatches.append(
                    (
                        header,
                        name,
                        "shape",
                        musl_table[name],
                        crabc_table[name],
                    )
                )
            else:
                probes.append(name)

        if not probes:
            continue
        entries = [(name, musl_table[name]) for name in probes]
        musl_expanded = expand_macros(args.cc, args.musl, header, entries)
        crabc_expanded = expand_macros(args.cc, args.crabc, header, entries)
        if musl_expanded is None or crabc_expanded is None:
            # One malformed macro can break a batch.  Fall back to one probe
            # per macro so one bad expansion does not hide the rest.
            for name in probes:
                musl_one = expand_macros(
                    args.cc, args.musl, header, [(name, musl_table[name])]
                )
                crabc_one = expand_macros(
                    args.cc, args.crabc, header, [(name, crabc_table[name])]
                )
                if musl_one is None or crabc_one is None:
                    skipped.append((header, name))
                elif musl_one[0] != crabc_one[0]:
                    mismatches.append(
                        (header, name, "expand", musl_one[0], crabc_one[0])
                    )
            continue
        for name, before, after in zip(probes, musl_expanded, crabc_expanded):
            if before != after:
                mismatches.append((header, name, "expand", before, after))

    real_mismatches = []
    for item in mismatches:
        header, name = item[0], item[1]
        reason = allowlist_match(allowlist, args.arch, header, name)
        if reason is not None:
            allowlisted += 1
            used_allowlist.add((header, name))
            continue
        real_mismatches.append(item)

    missing = sorted(musl_abi - crabc_abi)
    extra = sorted(crabc_abi - musl_abi)
    print(
        f"=== {args.arch}/{args.language}: headers={len(headers)} common_macros={common_count} "
        f"mismatches={len(real_mismatches)} allowlisted={allowlisted} "
        f"failures={len(failures)} skipped={len(skipped)}"
    )
    print(
        f"    coverage: missing_abi={len(missing)} "
        f"extra_abi={len(extra)} missing_headers={len(missing_headers)}"
    )
    coverage_regression = False
    coverage_key = f"{args.arch}-{args.language}"
    if coverage_key in coverage_baseline:
        baseline_missing, _ = coverage_baseline[coverage_key]
        if len(missing) > baseline_missing:
            coverage_regression = True
            print(
                f"    coverage regression: missing_abi {baseline_missing} -> "
                f"{len(missing)}"
            )
        elif len(missing) < baseline_missing:
            print(
                f"    coverage improved: missing_abi {baseline_missing} -> "
                f"{len(missing)} (update the baseline)"
            )

    if args.verbose:
        for header, name, kind, before, after in real_mismatches:
            print(f"  {header}: {name} [{kind}]")
            print(f"    musl : {before}")
            print(f"    crabc: {after}")
        for header, detail in failures:
            print(f"  {header}: {detail}")
        for header, detail in skipped:
            print(f"  skipped {header}: {detail}")
    else:
        for header, name, kind, _, _ in real_mismatches[: args.show]:
            print(f"  {header}: {name} [{kind}]")
        if len(real_mismatches) > args.show:
            print(f"  ... {len(real_mismatches) - args.show} more")
        for header, detail in failures[: args.show]:
            print(f"  {header}: {detail}")
        if skipped:
            print(f"  skipped {len(skipped)} header(s)")

    if used_allowlist:
        stale = [
            entry
            for entry in allowlist
            if entry[:3] not in used_allowlist
            and not any(
                fnmatch.fnmatchcase(args.arch, entry[0])
                and (entry[1], entry[2]) == (h, n)
                for h, n in used_allowlist
            )
        ]
        if stale and args.verbose:
            for entry in stale:
                print(f"  note: allowlist entry unused: {entry}")

    if missing:
        print(f"    missing ABI macros: {missing[:20]}")
    if missing_headers:
        print(f"    missing headers: {missing_headers}")
    if header_missing:
        for header, names in sorted(header_missing.items())[:10]:
            print(f"    {header}: missing {len(names)} ABI macros (e.g. {', '.join(names[:5])})")
    return 1 if (real_mismatches or failures or coverage_regression
                 or missing or missing_headers or header_missing) else 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", required=True, choices=["x86_64", "aarch64", "riscv64"])
    parser.add_argument("--language", choices=["c", "c++"], default="c")
    parser.add_argument("--musl", required=True, help="musl include directory")
    parser.add_argument("--crabc", default=os.path.join(os.path.dirname(__file__), "..", "include"))
    parser.add_argument("--cc", required=True)
    parser.add_argument("--allowlist", default=os.path.join(os.path.dirname(__file__), "macro-parity.allowlist"))
    parser.add_argument("--coverage-baseline", default=os.path.join(os.path.dirname(__file__), "macro-parity.coverage"))
    parser.add_argument("--show", type=int, default=20)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    global LANGUAGE
    LANGUAGE = args.language
    args.musl = os.path.abspath(args.musl)
    args.crabc = os.path.abspath(args.crabc)
    return compare(args)


if __name__ == "__main__":
    sys.exit(main())
