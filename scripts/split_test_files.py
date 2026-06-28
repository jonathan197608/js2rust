#!/usr/bin/env python3
"""Split large MDN test JS files into smaller chunks (~10 fragments each).

Usage: python scripts/split_test_files.py
"""

import re
import os

PROJECT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
JS_SRC_DIR = os.path.join(PROJECT_DIR, "examples", "mdn-test-project", "js_src")

# How many fragments per split file
FRAGMENTS_PER_FILE = 10


def parse_fragments(content):
    """Parse a test file into list of (fragment_num, fragment_text)."""
    # Match "// ---- fragment N ----" markers
    pattern = re.compile(r'// ---- fragment (\d+) ----\n')
    matches = list(pattern.finditer(content))

    fragments = []
    for i, match in enumerate(matches):
        fragment_num = int(match.group(1))
        start = match.start()
        # End is the next fragment marker, or the closing of the wrapper function
        if i + 1 < len(matches):
            end = matches[i + 1].start()
        else:
            end = len(content)
        fragment_text = content[start:end]
        fragments.append((fragment_num, fragment_text))

    return fragments


def split_file(filepath, base_name, category, total_fragments):
    """Split a test file into smaller files."""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    fragments = parse_fragments(content)
    print(f"  Parsed {len(fragments)} fragments from {os.path.basename(filepath)}")

    num_parts = (len(fragments) + FRAGMENTS_PER_FILE - 1) // FRAGMENTS_PER_FILE

    part_files = []
    for part_idx in range(num_parts):
        start = part_idx * FRAGMENTS_PER_FILE
        end = min(start + FRAGMENTS_PER_FILE, len(fragments))
        chunk = fragments[start:end]
        first_num = chunk[0][0]
        last_num = chunk[-1][0]

        part_name = f"test_{base_name}_part{part_idx + 1}"
        part_filename = f"{part_name}.js"
        part_path = os.path.join(JS_SRC_DIR, part_filename)

        # Generate the part file content
        lines = []
        lines.append("// Auto-generated from MDN JS Reference")
        lines.append(f"// Category: {category}")
        lines.append(f"// Fragments: {len(chunk)} (fragment {first_num}-{last_num})")
        lines.append(f"// Generated: 2026-06-28")
        lines.append("")
        lines.append(f"function {part_name}() {{")

        for _, fragment_text in chunk:
            # Replace the test function name reference in error messages
            # The original uses [testStatements], [testExpressions], [testBuiltins]
            fragment_text = fragment_text.replace(
                f"[test{category.capitalize()}]",
                f"[{part_name}]"
            )
            lines.append(fragment_text)

        lines.append("}")
        lines.append(f"module.exports = {{ {part_name} }};")
        lines.append("")

        with open(part_path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

        print(f"  Created: {part_filename} ({len(chunk)} fragments: {first_num}-{last_num})")
        part_files.append(part_filename)

    return part_files


def main():
    files_to_split = [
        ("test_statements.js", "statements", "statements", 44),
        ("test_expressions.js", "expressions", "expressions", 168),
        ("test_builtins.js", "builtins", "builtins", 228),
    ]

    all_part_files = []

    for filename, base_name, category, total in files_to_split:
        filepath = os.path.join(JS_SRC_DIR, filename)
        print(f"\nProcessing {filename} ({total} fragments):")
        part_files = split_file(filepath, base_name, category, total)
        all_part_files.extend(part_files)

    print(f"\n=== Summary ===")
    print(f"Total part files created: {len(all_part_files)}")
    print(f"Files:")
    for pf in sorted(all_part_files):
        print(f"  - {pf}")

    # Generate the js2rust.toml additional_js_files entries
    print(f"\n=== js2rust.toml additional_js_files entries ===")
    for pf in sorted(all_part_files):
        print(f'    "js_src/{pf}",')


if __name__ == "__main__":
    main()
