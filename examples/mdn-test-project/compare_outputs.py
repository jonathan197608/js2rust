#!/usr/bin/env python3
"""Compare Node.js vs Zig output for all 153 pass fragments."""
import json
import subprocess
import sys
import os

PROJECT_DIR = r"C:\Users\18988\RustroverProjects\js2rust\examples\mdn-test-project"
ZIG_BIN = r"C:\Users\18988\RustroverProjects\js2rust\target\debug\mdn-test-project.exe"
NODE_BIN = "node"

# Read pass fragments
with open(f"{PROJECT_DIR}/pass_fragments.json", "r") as f:
    data = json.load(f)

all_frags = []
for category in ["statements", "expressions", "builtins"]:
    for name in data[category]:
        all_frags.append(name)

print(f"Total fragments to compare: {len(all_frags)}")
print("=" * 60)

matches = 0
mismatches = 0
errors = 0
mismatch_list = []
error_list = []

for i, frag in enumerate(all_frags):
    # Run Node.js using the .node.js reference file (has try/catch for expected errors)
    try:
        node_result = subprocess.run(
            [NODE_BIN, f"js_src/{frag}.node.js"],
            capture_output=True, text=True, timeout=10,
            cwd=PROJECT_DIR
        )
        node_out = node_result.stdout.strip()
        node_err = node_result.stderr.strip()
        if node_result.returncode != 0:
            print(f"[{i+1}/{len(all_frags)}] {frag}: NODE ERROR - {node_err[:100]}")
            errors += 1
            error_list.append((frag, "node_error", node_err[:200]))
            continue
    except subprocess.TimeoutExpired:
        print(f"[{i+1}/{len(all_frags)}] {frag}: NODE TIMEOUT")
        errors += 1
        error_list.append((frag, "node_timeout", ""))
        continue

    # Run Zig
    try:
        zig_result = subprocess.run(
            [ZIG_BIN, frag],
            capture_output=True, text=True, timeout=10
        )
        # Zig console.log writes to stderr
        zig_out = zig_result.stderr.strip()
        if zig_result.returncode != 0:
            print(f"[{i+1}/{len(all_frags)}] {frag}: ZIG ERROR - {zig_out[:100]}")
            errors += 1
            error_list.append((frag, "zig_error", zig_out[:200]))
            continue
    except subprocess.TimeoutExpired:
        print(f"[{i+1}/{len(all_frags)}] {frag}: ZIG TIMEOUT")
        errors += 1
        error_list.append((frag, "zig_timeout", ""))
        continue

    # Compare
    if node_out == zig_out:
        matches += 1
        # Only print first 10 matches to avoid spam
        if matches <= 10:
            print(f"[{i+1}/{len(all_frags)}] {frag}: MATCH")
    else:
        mismatches += 1
        mismatch_list.append((frag, node_out, zig_out))
        print(f"[{i+1}/{len(all_frags)}] {frag}: MISMATCH")
        print(f"  Node: {node_out[:200]}")
        print(f"  Zig:  {zig_out[:200]}")

print("=" * 60)
print(f"Results: {matches} match, {mismatches} mismatch, {errors} error (total: {len(all_frags)})")

if mismatch_list:
    print("\n--- Mismatch Details ---")
    for frag, node_out, zig_out in mismatch_list:
        print(f"\n{frag}:")
        print(f"  Node: {repr(node_out[:300])}")
        print(f"  Zig:  {repr(zig_out[:300])}")

if error_list:
    print("\n--- Error Details ---")
    for frag, err_type, err_msg in error_list:
        print(f"  {frag}: {err_type} - {err_msg[:150]}")

# Save results
results = {
    "total": len(all_frags),
    "matches": matches,
    "mismatches": mismatches,
    "errors": errors,
    "mismatch_list": [(f, n, z) for f, n, z in mismatch_list],
    "error_list": [(f, t, m) for f, t, m in error_list],
}
with open(f"{PROJECT_DIR}/comparison_results.json", "w") as f:
    json.dump(results, f, indent=2)

print(f"\nResults saved to comparison_results.json")
