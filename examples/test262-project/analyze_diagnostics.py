#!/usr/bin/env python3
"""Parse test262 diagnostics.json and generate a feature gap report."""

import json
import re
from collections import defaultdict

DIAG_FILE = r"C:\Users\18988\RustroverProjects\js2rust\examples\test262-project\.js2zig-cache\app\diagnostics.json"

with open(DIAG_FILE, "r", encoding="utf-8") as f:
    diags = json.load(f)

# Parse each diagnostic entry
entries = []
for d in diags:
    # Pattern: "filename.js: TYPE - message"
    m = re.match(r'^(.+?\.(?:js|zig)):\s+(COMPILE_ERROR|ERROR|WARNING|INFO)\s+-\s+(.+)$', d)
    if m:
        entries.append({
            "file": m.group(1),
            "type": m.group(2),
            "message": m.group(3),
        })
    else:
        # Non-standard format (e.g. closure warnings)
        m2 = re.match(r'^(.+?\.(?:js|zig)):\s+(.+)$', d)
        if m2:
            entries.append({
                "file": m2.group(1),
                "type": "WARNING",
                "message": m2.group(2),
            })

# Categorize by error type
by_type = defaultdict(lambda: {"count": 0, "files": set()})
for e in entries:
    # Normalize the message to group similar errors
    msg = e["message"]
    if "Unsupported NewExpression: new Test262Error()" in msg:
        category = "Unsupported NewExpression: new Test262Error()"
    elif "Unsupported NewExpression: new Object()" in msg:
        category = "Unsupported NewExpression: new Object()"
    elif "eval() is not supported" in msg:
        category = "eval() is not supported"
    elif "nested closure capture is not supported" in msg:
        category = "nested closure capture is not supported"
    elif "`this` used outside of a class method" in msg:
        category = "`this` used outside of a class method"
    elif "computed property key" in msg:
        category = "computed property key `{ [expr]: value }` not supported"
    elif "must be initialized (strict type system)" in msg:
        category = "Variable must be initialized (strict type system)"
    elif "captures" in msg and "not referenced in the body" in msg:
        category = "closure captures variable but not referenced in body"
    else:
        category = msg[:80]

    key = (e["type"], category)
    by_type[key]["count"] += 1
    by_type[key]["files"].add(e["file"])

# Get unique files
all_files = set(e["file"] for e in entries)
files_with_compile_errors = set(e["file"] for e in entries if e["type"] == "COMPILE_ERROR")
files_with_errors = set(e["file"] for e in entries if e["type"] == "ERROR")
files_with_warnings = set(e["file"] for e in entries if e["type"] == "WARNING")

# Count total test files (excluding runtime.js and app.js)
test_files = [f for f in all_files if f.startswith("test262_")]
clean_files = [f for f in all_files if f not in files_with_compile_errors and f not in files_with_errors]

print("=" * 80)
print("TEST262 FEATURE GAP REPORT")
print("=" * 80)
print()
print(f"Total diagnostics: {len(entries)}")
print(f"Unique test files with any diagnostic: {len(set(f for f in all_files if f.startswith('test262_')))}")
print(f"Files with COMPILE_ERROR (non-blocking @compileError): {len(set(f for f in files_with_compile_errors if f.startswith('test262_')))}")
print(f"Files with ERROR (hard error, Zig not generated): {len(set(f for f in files_with_errors if f.startswith('test262_')))}")
print(f"Clean files (no diagnostics): {len(clean_files)}")
print()

print("-" * 80)
print("COMPILE_ERROR categories (non-blocking @compileError in generated Zig):")
print("-" * 80)
for (etype, cat), info in sorted(by_type.items(), key=lambda x: -x[1]["count"]):
    if etype == "COMPILE_ERROR":
        test_count = len([f for f in info["files"] if f.startswith("test262_")])
        print(f"  [{info['count']:4d} occurrences, {test_count:2d} tests] {cat}")

print()
print("-" * 80)
print("ERROR categories (hard errors, Zig file not generated for affected constructs):")
print("-" * 80)
for (etype, cat), info in sorted(by_type.items(), key=lambda x: -x[1]["count"]):
    if etype == "ERROR":
        test_count = len([f for f in info["files"] if f.startswith("test262_")])
        print(f"  [{info['count']:4d} occurrences, {test_count:2d} tests] {cat}")

print()
print("-" * 80)
print("WARNING categories (non-blocking diagnostics):")
print("-" * 80)
for (etype, cat), info in sorted(by_type.items(), key=lambda x: -x[1]["count"]):
    if etype == "WARNING":
        test_count = len([f for f in info["files"] if f.startswith("test262_")])
        print(f"  [{info['count']:4d} occurrences, {test_count:2d} tests] {cat}")

print()
print("-" * 80)
print("DETAILED FILE-BY-FILE BREAKDOWN")
print("-" * 80)
file_diags = defaultdict(list)
for e in entries:
    if e["file"].startswith("test262_"):
        file_diags[e["file"]].append(e)

for fname in sorted(file_diags.keys()):
    diags_for_file = file_diags[fname]
    compile_errs = [d for d in diags_for_file if d["type"] == "COMPILE_ERROR"]
    hard_errs = [d for d in diags_for_file if d["type"] == "ERROR"]
    warnings = [d for d in diags_for_file if d["type"] == "WARNING"]

    # Get unique categories
    ce_cats = set()
    for d in compile_errs:
        if "new Test262Error()" in d["message"]:
            ce_cats.add("new Test262Error()")
        elif "new Object()" in d["message"]:
            ce_cats.add("new Object()")
        elif "eval()" in d["message"]:
            ce_cats.add("eval()")
        elif "nested closure" in d["message"]:
            ce_cats.add("nested closure capture")
        elif "`this`" in d["message"]:
            ce_cats.add("`this` outside class")

    he_cats = set()
    for d in hard_errs:
        if "computed property" in d["message"]:
            he_cats.add("computed property key")
        elif "must be initialized" in d["message"]:
            he_cats.add("var must be initialized")
        elif "`this`" in d["message"]:
            he_cats.add("`this` outside class")

    short_name = fname.replace("test262_language_expressions_addition_", "")
    status = "CLEAN" if not ce_cats and not he_cats else "ISSUES"
    print(f"  {short_name:40s} {status}")
    if ce_cats:
        print(f"    COMPILE_ERROR: {', '.join(sorted(ce_cats))}")
    if he_cats:
        print(f"    ERROR:         {', '.join(sorted(he_cats))}")

# Summary table
print()
print("=" * 80)
print("SUMMARY TABLE")
print("=" * 80)
print(f"{'Feature Gap':<45} {'Type':<15} {'Tests':>6} {'Occurrences':>12}")
print("-" * 80)
for (etype, cat), info in sorted(by_type.items(), key=lambda x: -x[1]["count"]):
    test_count = len([f for f in info["files"] if f.startswith("test262_")])
    if test_count > 0:
        print(f"{cat:<45} {etype:<15} {test_count:>6} {info['count']:>12}")
