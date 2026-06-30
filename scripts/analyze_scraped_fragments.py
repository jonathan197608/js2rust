#!/usr/bin/env python3
"""
Analyze scraped MDN test fragments against js2zig not-implemented features.
Output: summary of how many fragments are usable vs filtered out.
"""

import re
import sys
from pathlib import Path
from collections import defaultdict

SCRAPED_DIR = Path(__file__).resolve().parent.parent / "examples" / "mdn-test-project" / "js_src"
FILES = ["test_statements.js", "test_expressions.js", "test_builtins.js"]

# ── Categories of not-implemented features ──

# Category 1: Syntax the scraper's UNSUPPORTED_RE caught (re-check)
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

# Category 2: Regular expression literals (js2zig only supports host func, not literals well)
# Regex with flags that aren't well-supported
REGEX_LITERAL = re.compile(r'/[^/]+/[gimsuvy]*')

# Category 4: let/const destructuring (supported but sometimes problematic)
DESTRUCTURING = re.compile(r'^\s*(?:const|let|var)\s*\{[^}]+\}\s*=|^\s*(?:const|let|var)\s*\[[^\]]+\]\s*=', re.MULTILINE)

# Category 5: Promise usage
PROMISE = re.compile(r'\bnew\s+Promise\b|\.then\s*\(|\.catch\s*\(|Promise\.all|Promise\.race|Promise\.resolve|Promise\.reject|Promise\.any')

# Category 6: instanceof
INSTANCEOF = re.compile(r'\binstanceof\b')

# Category 7: class expression (not class declaration)
CLASS_EXPRESSION = re.compile(r'=\s*class\b')

# Category 8: Generator/async generator patterns
GENERATOR = re.compile(r'\bfunction\s*\*\b|\basync\s+function\s*\*\b')

# Category 9: for-await-of (should be caught by scraper but re-check)
FOR_AWAIT = re.compile(r'for\s+await\s*\(')

# Category 10: arguments object (should be caught by scraper but re-check)
ARGUMENTS_OBJ = re.compile(r'\barguments\b')

# Category 11: static {} block
STATIC_BLOCK = re.compile(r'static\s*\{')

# Category 12: label usage that may cause issues
# (labels themselves are fine, skip)

# Category 13: ErroredRegExp.js type patterns — regex error tests
REGEX_ERROR_PATTERNS = re.compile(r'/\b[a-zA-Z_]+\s*\.\s*[a-zA-Z_]+\(', re.S)  # heuristic

# Category 14: switch with string cases (problematic in Zig)
SWITCH_STRING = re.compile(r'switch\s*\([^)]*\)\s*\{[^}]*case\s*"[^"]*"')

# Category 15: throw/catch with unsupported Error subtypes
UNSUPPORTED_ERROR = re.compile(r'\bEvalError\b|\bInternalError\b')

# ── All filter patterns ──
FILTER_PATTERNS = [
    ("Scraper already filtered (generators/proxy/etc)", SCRAPER_FILTERED),
    ("Promise usage", PROMISE),
    ("instanceof (not implemented)", INSTANCEOF),
    ("Class expression", CLASS_EXPRESSION),
    ("Generator function", GENERATOR),
    ("for-await-of", FOR_AWAIT),
    ("Switch with string cases", SWITCH_STRING),
    ("Unsupported Error type", UNSUPPORTED_ERROR),
]

def parse_fragments(filepath: Path) -> list[str]:
    """Extract individual fragments from test file."""
    content = filepath.read_text(encoding="utf-8")
    # Fragments are separated by "// ---- fragment N ----"
    parts = re.split(r'\s*//\s*-+\s*fragment\s*\d+\s*-+\s*', content)
    # First part is the function header, last part is closing brace + module.exports
    fragments = []
    for i, part in enumerate(parts):
        if i == 0:
            continue  # skip header
        # Extract code between try { ... } catch
        # Remove the try { and } catch { ... } wrapping
        code = part.strip()
        if not code:
            continue
        
        # Remove try { and catch { ... } wrapping
        m = re.search(r'try\s*\{(.*)\}\s*catch\s*\(.*?\)\s*\{.*?\}', code, re.DOTALL)
        if m:
            inner = m.group(1).strip()
        else:
            inner = code
        
        # Unwrap async IIFE wrapper if present
        inner = re.sub(r'^\s*\(async\s*\(\)\s*=>\s*\{', '', inner)
        inner = re.sub(r'\}\)\(\s*\)\s*;\s*$', '', inner)
        
        inner = inner.strip()
        if inner:
            fragments.append(inner)
    return fragments


def analyze_fragment(frag: str) -> list[str]:
    """Check fragment against all filter patterns. Returns list of reasons."""
    reasons = []
    for name, pattern in FILTER_PATTERNS:
        if pattern.search(frag):
            reasons.append(name)
    return reasons


def main():
    results = {}
    total_fragments = 0
    filtered_counts = defaultdict(int)
    filtered_out = 0
    kept = 0
    kept_by_file = defaultdict(list)
    filtered_by_file = defaultdict(list)
    
    for fname in FILES:
        fpath = SCRAPED_DIR / fname
        if not fpath.exists():
            print(f"  ✗ {fname} not found")
            continue
        
        fragments = parse_fragments(fpath)
        total = len(fragments)
        total_fragments += total
        print(f"\n{'='*60}")
        print(f"File: {fname} ({total} fragments)")
        print(f"{'='*60}")
        
        file_kept = 0
        file_filtered = 0
        
        for idx, frag in enumerate(fragments):
            reasons = analyze_fragment(frag)
            if reasons:
                file_filtered += 1
                filtered_out += 1
                for r in reasons:
                    filtered_counts[r] += 1
                filtered_by_file[fname].append((idx, reasons, frag[:80].replace('\n', '\\n')))
            else:
                file_kept += 1
                kept += 1
                kept_by_file[fname].append((idx, frag[:80].replace('\n', '\\n')))
        
        print(f"  Kept: {file_kept}, Filtered: {file_filtered}")
    
    # ── Summary ──
    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"Total fragments scraped: {total_fragments}")
    print(f"Kept (usable):           {kept}")
    print(f"Filtered out:            {filtered_out}")
    print(f"Keep rate:               {kept/total_fragments*100:.1f}%")
    
    print(f"\nFilter reasons:")
    for name, count in sorted(filtered_counts.items(), key=lambda x: -x[1]):
        print(f"  {name}: {count}")
    
    # ── Per-file detail ──
    print(f"\n{'='*60}")
    print(f"FILTERED FRAGMENTS (by file)")
    print(f"{'='*60}")
    for fname in FILES:
        items = filtered_by_file.get(fname, [])
        if items:
            print(f"\n--- {fname} ({len(items)} filtered) ---")
            for idx, reasons, preview in items:
                print(f"  Fragment {idx}: {', '.join(reasons)}")
                print(f"    Preview: {preview}")
    
    # ── Kept breakdown ──
    print(f"\n{'='*60}")
    print(f"KEPT FRAGMENTS BREAKDOWN")
    print(f"{'='*60}")
    for fname in FILES:
        items = kept_by_file.get(fname, [])
        print(f"\n--- {fname} ({len(items)} kept) ---")
        for idx, preview in items[:10]:  # show first 10
            print(f"  Fragment {idx}: {preview}")
        if len(items) > 10:
            print(f"  ... and {len(items)-10} more")
    
    results["total"] = total_fragments
    results["kept"] = kept
    results["filtered"] = filtered_out
    results["reasons"] = dict(filtered_counts)
    return results


if __name__ == "__main__":
    main()
