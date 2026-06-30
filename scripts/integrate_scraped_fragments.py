#!/usr/bin/env python3
"""
Integrate filtered MDN scraped fragments into mdn-test-project.

Reads the 3 scraped JS test files, filters out fragments with
not-implemented features, splits into parts (~10 fragments each),
and writes clean part files + updates js2rust.toml and main.rs.
"""
import re
import sys
from pathlib import Path
from collections import defaultdict

PROJECT_DIR = Path(__file__).resolve().parent.parent / "examples" / "mdn-test-project"
JS_SRC = PROJECT_DIR / "js_src"

# The 3 scraped source files
SCRAPED_FILES = {
    "test_statements.js": ("statements", "testStatements"),
    "test_expressions.js": ("expressions", "testExpressions"),
    "test_builtins.js": ("builtins", "testBuiltins"),
}

# ── Filter patterns (same as analyze_scraped_fragments.py, minus BigInt) ──

SCRAPER_FILTERED = re.compile(r"""
    function\s*\*\s*\(    |
    async\s+function\s*\*\s*\( |
    \byield\b             |
    for\s+await\s*\(\s*   |
    \bimport\s*\(         |
    \bimport\.meta\b      |
    \bnew\.target\b       |
    \?\\.                  |
    #\w+                  |
    \bextends\s+\w+       |
    \bwith\s*\(           |
    \bdebugger\b          |
    \barguments\b         |
    \bWeakMap\b           |
    \bWeakSet\b           |
    \bProxy\b             |
    \bReflect\b           |
    \bIntl\.              |
    \bAtomics\b           |
    \bSharedArrayBuffer\b |
    \bFinalizationRegistry\b |
    \bWeakRef\b           |
    \.toReversed\s*\(     |
    \.toSorted\s*\(       |
    \.toSpliced\s*\(      |
    \.with\s*\(           |
    \.groupBy\s*\(        |
    \.getOwnPropertySymbols\s*\( |
    \bwindow\b            |
    \bdocument\b          |
    \bXMLHttpRequest\b    |
    \bfetch\s*\(
""", re.VERBOSE)

PROMISE = re.compile(r'\bnew\s+Promise\b|\.then\s*\(|\.catch\s*\(|Promise\.all|Promise\.race|Promise\.resolve|Promise\.reject|Promise\.any')
INSTANCEOF = re.compile(r'\binstanceof\b')
SWITCH_STRING = re.compile(r'switch\s*\([^)]*\)\s*\{[^}]*case\s*"[^"]*"')
UNSUPPORTED_ERROR = re.compile(r'\bEvalError\b|\bInternalError\b')

FILTER_PATTERNS = [
    ("Scraper already filtered", SCRAPER_FILTERED),
    ("Promise usage", PROMISE),
    ("instanceof", INSTANCEOF),
    ("Switch with string cases", SWITCH_STRING),
    ("Unsupported Error type", UNSUPPORTED_ERROR),
]

FRAGMENTS_PER_PART = 10


def should_filter(code: str) -> list[str]:
    """Return list of reasons to filter, empty if clean."""
    reasons = []
    for name, pattern in FILTER_PATTERNS:
        if pattern.search(code):
            reasons.append(name)
    return reasons


def parse_scraped_file(filepath: Path) -> list[tuple[int, str, str]]:
    """Parse a scraped JS file into fragments.
    Returns list of (fragment_num, full_fragment_block, inner_code).
    full_fragment_block = the entire '// ---- fragment N ---- ... try {...} catch {...}' block.
    inner_code = just the code inside the try block (without curly braces wrappers).
    """
    content = filepath.read_text(encoding="utf-8")
    # Split on fragment markers
    parts = re.split(r'//\s*-+\s*fragment\s+(\d+)\s*-+\s*\n', content)
    # parts[0] = header + function wrapper opening
    # parts[1] = fragment number, parts[2] = fragment content, parts[3] = next num, etc.
    
    fragments = []
    for i in range(1, len(parts), 2):
        frag_num = int(parts[i])
        frag_content = parts[i + 1] if i + 1 < len(parts) else ""
        
        # Extract inner code from try {{ ... }} catch (e) {{ ... }}
        m = re.search(r'try\s*\{\{(.*?)\}\}\s*catch\s*\(.*?\)\s*\{\{.*?\}\}',
                      frag_content, re.DOTALL)
        if m:
            inner_code = m.group(1).strip()
        else:
            inner_code = frag_content.strip()
        
        full_block = f"// ---- fragment {frag_num} ----\n{frag_content.strip()}"
        fragments.append((frag_num, full_block, inner_code))
    
    return fragments


def generate_part_file(category: str, part_index: int, fragments: list[tuple[int, str, str]],
                       func_name: str, first_frag: int, last_frag: int) -> str:
    """Generate content for a single part file."""
    lines = []
    lines.append("// Auto-generated from MDN JS Reference")
    lines.append(f"// Category: {category}")
    lines.append(f"// Fragments: {len(fragments)} (fragment {first_frag}-{last_frag})")
    lines.append("// Generated: 2026-06-30")
    lines.append("")
    lines.append(f"function {func_name}() {{")
    
    for frag_num, full_block, _ in fragments:
        # Update the function name in the catch block
        block = full_block.replace(f"[test{category.capitalize()}]",
                                   f"[{func_name}]")
        lines.append(block)
        lines.append("")
    
    lines.append("}")
    lines.append(f"module.exports = {{ {func_name} }};")
    lines.append("")
    
    return "\n".join(lines)


def main():
    all_part_files = {}
    filter_report = defaultdict(list)
    total_kept = 0
    total_filtered = 0
    
    for src_file, (category, base_func) in SCRAPED_FILES.items():
        fpath = JS_SRC / src_file
        if not fpath.exists():
            print(f"SKIP: {src_file} not found")
            continue
        
        fragments = parse_scraped_file(fpath)
        print(f"\n{'='*60}")
        print(f"Processing {src_file}: {len(fragments)} fragments")
        
        kept_frags = []
        file_filtered = 0
        
        for frag_num, full_block, inner_code in fragments:
            reasons = should_filter(inner_code)
            if reasons:
                file_filtered += 1
                filter_report[src_file].append((frag_num, reasons))
                print(f"  FILTER fragment {frag_num}: {', '.join(reasons)}")
            else:
                kept_frags.append((frag_num, full_block, inner_code))
        
        print(f"  Kept: {len(kept_frags)}, Filtered: {file_filtered}")
        total_kept += len(kept_frags)
        total_filtered += file_filtered
        
        # Split into parts
        part_files_for_category = []
        for part_idx in range(0, len(kept_frags), FRAGMENTS_PER_PART):
            part_frags = kept_frags[part_idx:part_idx + FRAGMENTS_PER_PART]
            part_num = part_idx // FRAGMENTS_PER_PART + 1
            first_frag = part_frags[0][0]
            last_frag = part_frags[-1][0]
            
            func_name = f"test_{category}_part{part_num}"
            file_name = f"test_{category}_part{part_num}.js"
            
            content = generate_part_file(category, part_num, part_frags,
                                         func_name, first_frag, last_frag)
            
            out_path = JS_SRC / file_name
            out_path.write_text(content, encoding="utf-8")
            print(f"  WROTE {file_name} ({len(part_frags)} fragments: {first_frag}-{last_frag})")
            
            part_files_for_category.append((file_name, func_name))
        
        all_part_files[category] = part_files_for_category
    
    # ── Generate js2rust.toml ──
    print(f"\n{'='*60}")
    print("Generating js2rust.toml")
    
    toml_lines = []
    toml_lines.append('[project]')
    toml_lines.append('js_file = "js_src/app.js"')
    toml_lines.append('additional_js_files = [')
    toml_lines.append('    "js_src/test_minimal.js",')
    toml_lines.append('    "js_src/test_simple_ternary.js",')
    toml_lines.append('    "js_src/test_ternary_concat.js",')
    toml_lines.append('')
    
    for category in ["statements", "expressions", "builtins"]:
        parts = all_part_files.get(category, [])
        if not parts:
            continue
        count = len(parts)
        # Calculate total fragments for this category
        total_frags = sum(
            len(parse_scraped_file(JS_SRC / f"test_{category}.js")) 
            for f in [f"test_{category}.js"] if (JS_SRC / f"test_{category}.js").exists()
        )
        toml_lines.append(f'    # === {category.capitalize()} ({count} parts, {total_kept_for_cat(JS_SRC, category)} fragments) ===')
        for file_name, _ in parts:
            toml_lines.append(f'    "js_src/{file_name}",')
        toml_lines.append('')
    
    # Remove the trailing blank line and last comma fix
    toml_text = '\n'.join(toml_lines)
    toml_text = re.sub(r',\n\s*\n', ',\n', toml_text)
    # Ensure last entry has trailing comma and add closing bracket
    toml_text = toml_text.rstrip() + '\n]'
    
    toml_path = PROJECT_DIR / "js2rust.toml"
    toml_path.write_text(toml_text + '\n', encoding="utf-8")
    print(f"WROTE {toml_path}")
    
    # ── Generate main.rs ──
    print(f"\n{'='*60}")
    print("Generating main.rs")
    
    main_lines = []
    main_lines.append('// src/main.rs — MDN JS Reference tests (auto-generated)')
    main_lines.append('use js2rust_bridge::js2rust_bridge;')
    main_lines.append('')
    main_lines.append('js2rust_bridge!();')
    main_lines.append('')
    main_lines.append('fn main() {')
    main_lines.append('    js2rust_init();')
    main_lines.append('    println!("=== MDN JS Reference Tests ===");')
    main_lines.append('')
    main_lines.append('    // Minimal tests')
    main_lines.append('    println!("\\n=== MINIMAL ===");')
    main_lines.append('    let _ = testMinimal_app();')
    main_lines.append('')
    
    for category in ["statements", "expressions", "builtins"]:
        parts = all_part_files.get(category, [])
        if not parts:
            continue
        main_lines.append(f'    // {category.capitalize()} tests')
        main_lines.append(f'    println!("\\n=== {category.upper()} ===");')
        for _, func_name in parts:
            main_lines.append(f'    let _ = {func_name}_app();')
        main_lines.append('')
    
    main_lines.append('    js2rust_deinit();')
    main_lines.append('    println!("\\n=== All tests done ===");')
    main_lines.append('}')
    main_lines.append('')
    
    main_path = PROJECT_DIR / "src" / "main.rs"
    main_path.write_text('\n'.join(main_lines), encoding="utf-8")
    print(f"WROTE {main_path}")
    
    # ── Summary ──
    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"Total fragments: {total_kept + total_filtered}")
    print(f"Kept (integrated): {total_kept}")
    print(f"Filtered out: {total_filtered}")
    print(f"Integration rate: {total_kept/(total_kept+total_filtered)*100:.1f}%")
    print(f"\nPart files generated:")
    for category in ["statements", "expressions", "builtins"]:
        parts = all_part_files.get(category, [])
        for file_name, func_name in parts:
            print(f"  js_src/{file_name}")


def total_kept_for_cat(js_src, category):
    """Get count of kept fragments for a category (for toml comments)."""
    src_file = js_src / f"test_{category}.js"
    if not src_file.exists():
        return "?"
    fragments = parse_scraped_file(src_file)
    kept = sum(1 for _, _, code in fragments if not should_filter(code))
    return str(kept)


if __name__ == "__main__":
    main()
