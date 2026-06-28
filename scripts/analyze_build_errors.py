#!/usr/bin/env python3
"""Analyze Zig/codegen compilation errors from mdn-test-project build output.

Usage:
    cargo build -p mdn-test-project 2>&1 | python scripts/analyze_build_errors.py
    python scripts/analyze_build_errors.py build_output.txt
"""

import re
import sys
from collections import defaultdict
from pathlib import Path


def parse_codegen_errors(output):
    """Extract codegen (JS transpile) errors grouped by file."""
    errors = defaultdict(list)
    # Pattern: "skip 'filename': N codegen error(s)" followed by indented messages
    skip_pat = re.compile(r"skip '([^']+)': (\d+) codegen error")
    msg_pat = re.compile(r"^\s{4}(.+)$")

    current_file = None
    for line in output.split("\n"):
        m = skip_pat.search(line)
        if m:
            current_file = m.group(1)
            continue
        if current_file:
            m2 = msg_pat.match(line)
            if m2 and not m2.group(1).startswith("skip"):
                errors[current_file].append(m2.group(1))
            elif not line.startswith("    "):
                current_file = None
    return dict(errors)


def parse_zig_errors(output):
    """Extract Zig compilation errors grouped by source file."""
    errors = defaultdict(list)
    # Pattern: "filename.zig:line:col: error: message"
    zig_pat = re.compile(r"^([^:\s]+\.zig):(\d+):(\d+): error: (.+)$")

    for line in output.split("\n"):
        m = zig_pat.match(line)
        if m:
            filename = m.group(1)
            msg = f"line {m.group(2)}:{m.group(3)} - {m.group(4)}"
            errors[filename].append(msg)
    return dict(errors)


def classify_error(msg):
    """Classify a single error message into a root cause category."""
    msg_lower = msg.lower()

    if "undefined" in msg_lower or "not declared" in msg_lower or "undeclared" in msg_lower:
        return "undefined/undeclared symbol"
    if "no field" in msg_lower or "no member" in msg_lower:
        return "missing field/member"
    if "expected" in msg_lower and ("type" in msg_lower or "struct" in msg_lower):
        return "type mismatch"
    if "no function" in msg_lower or "no method" in msg_lower:
        return "missing function/method"
    if "cannot convert" in msg_lower or "cannot coerce" in msg_lower:
        return "type coercion failure"
    if "not supported" in msg_lower:
        return "unsupported feature"
    if "arrow" in msg_lower or "closure" in msg_lower:
        return "arrow function/closure"
    if "spread" in msg_lower:
        return "spread operator"
    if "template" in msg_lower or "`" in msg:
        return "template literal"
    if "regexp" in msg_lower or "regex" in msg_lower:
        return "regexp"
    if "bigint" in msg_lower:
        return "BigInt"
    if "symbol" in msg_lower:
        return "Symbol"
    if "promise" in msg_lower or "async" in msg_lower or "await" in msg_lower:
        return "Promise/async"
    if "class" in msg_lower:
        return "class"
    if "import" in msg_lower or "export" in msg_lower or "module" in msg_lower:
        return "module system"
    if "iife" in msg_lower:
        return "IIFE"
    if "new " in msg_lower and "expression" in msg_lower:
        return "new expression"
    if "destructur" in msg_lower:
        return "destructuring"
    if "for" in msg_lower and ("of" in msg_lower or "in" in msg_lower):
        return "for-of/in"
    return "other"


def main():
    if len(sys.argv) > 1:
        text = Path(sys.argv[1]).read_text(encoding="utf-8")
    else:
        text = sys.stdin.read()

    codegen_errs = parse_codegen_errors(text)
    zig_errs = parse_zig_errors(text)

    total_codegen_files = len(codegen_errs)
    total_codegen_msgs = sum(len(v) for v in codegen_errs.values())
    total_zig_files = len(zig_errs)
    total_zig_msgs = sum(len(v) for v in zig_errs.values())

    print("=" * 70)
    print("BUILD ERROR ANALYSIS REPORT")
    print("=" * 70)
    print(f"\nCodegen (JS->Zig) errors: {total_codegen_msgs} errors in {total_codegen_files} files")
    print(f"Zig compilation errors:    {total_zig_msgs} errors in {total_zig_files} files")

    # Codegen errors
    if codegen_errs:
        print("\n--- Codegen Errors by File ---")
        for fname in sorted(codegen_errs.keys()):
            errs = codegen_errs[fname]
            print(f"\n  {fname} ({len(errs)} errors):")
            for e in errs[:5]:
                print(f"    - {e}")
            if len(errs) > 5:
                print(f"    ... and {len(errs) - 5} more")

    # Zig errors
    if zig_errs:
        print("\n--- Zig Compilation Errors by File ---")
        for fname in sorted(zig_errs.keys()):
            errs = zig_errs[fname]
            print(f"\n  {fname} ({len(errs)} errors):")
            for e in errs[:5]:
                print(f"    - {e}")
            if len(errs) > 5:
                print(f"    ... and {len(errs) - 5} more")

    # Root cause classification
    all_errs = []
    for errs in codegen_errs.values():
        all_errs.extend(errs)
    for errs in zig_errs.values():
        all_errs.extend(errs)

    if all_errs:
        causes = defaultdict(int)
        for e in all_errs:
            cat = classify_error(e)
            causes[cat] += 1

        print("\n--- Root Cause Distribution ---")
        for cat, count in sorted(causes.items(), key=lambda x: -x[1]):
            pct = count * 100.0 / len(all_errs)
            bar = "#" * int(pct / 2)
            print(f"  {cat:30s} {count:4d} ({pct:5.1f}%) {bar}")

    # Pass/Fail summary per file category
    total_parts = {
        "statements": 5, "expressions": 17, "builtins": 23
    }
    print("\n--- Success Rate by Category ---")
    for cat, total in total_parts.items():
        failed_codegen = sum(1 for f in codegen_errs if cat in f.lower())
        failed_zig = sum(1 for f in zig_errs if cat in f.lower())
        # Files that appear in zig_errs but NOT in codegen_errs passed codegen
        passed_codegen = total - failed_codegen
        passed_zig = total - failed_zig
        print(f"  {cat:15s}: {passed_codegen}/{total} codegen OK, {passed_zig}/{total} Zig OK")

    print("\nDone.")


if __name__ == "__main__":
    main()
