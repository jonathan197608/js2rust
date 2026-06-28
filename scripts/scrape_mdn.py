#!/usr/bin/env python3
"""
scrape_mdn.py  –  Generate JS test files from MDN examples.

What it does
-------------
1. Fetches curated MDN pages (statements + built-ins only; operator
   pages on MDN are conceptual and have no runnable console.log examples).
2. Extracts JS code blocks that contain console.log(...) calls.
3. Converts each snippet:
      console.log(x)  →  _out.push(x);
   Handles multi-line console.log calls by counting parentheses.
4. Merges ALL snippets of the same category into ONE exported function:
      export function testExpressions() { ... return _out; }
      export function testStatements()  { ... return _out; }
      export function testBuiltins()    { ... return _out; }
5. Writes 3 JS files + a runner (run_all.js) to examples/mdn-test-project/js_src/.
6. In run_all.js, overrides console.log to capture output (double protection).

Operator tests are added MANUALLY (see MANUAL_OPS below).

Usage
------
    python scrape_mdn.py          # full run  → write JS files
    python scrape_mdn.py --dry   # print URLs, no network
    python scrape_mdn.py --test  # fetch 4 pages, print generated JS
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import urljoin

try:
    import requests
    from bs4 import BeautifulSoup
except ImportError:
    print("Missing deps:  pip install requests beautifulsoup4")
    sys.exit(1)

# ── Paths ─────────────────────────────────────────────────────────────
SCRIPT_DIR = Path(__file__).resolve().parent
OUTPUT_DIR  = SCRIPT_DIR.parent / "examples" / "mdn-test-project" / "js_src"
MDN = "https://developer.mozilla.org"

# ── Curated MDN URLs ─────────────────────────────────────────────────
SUPPORTED = [
    # ── Statements ─────────────────────────────────────────────────
    ("/Statements/if...else",           "statement"),
    ("/Statements/switch",              "statement"),
    ("/Statements/for",                  "statement"),
    ("/Statements/for...of",             "statement"),
    ("/Statements/for...in",             "statement"),
    ("/Statements/while",               "statement"),
    ("/Statements/do...while",           "statement"),
    ("/Statements/break",               "statement"),
    ("/Statements/continue",            "statement"),
    ("/Statements/throw",               "statement"),
    ("/Statements/try...catch",          "statement"),
    ("/Statements/function",            "statement"),
    # ── Built-in: Math ───────────────────────────────────────────
    ("/Global_Objects/Math/abs",          "builtin"),
    ("/Global_Objects/Math/ceil",        "builtin"),
    ("/Global_Objects/Math/floor",       "builtin"),
    ("/Global_Objects/Math/round",       "builtin"),
    ("/Global_Objects/Math/sqrt",        "builtin"),
    ("/Global_Objects/Math/random",      "builtin"),
    ("/Global_Objects/Math/pow",         "builtin"),
    ("/Global_Objects/Math/max",         "builtin"),
    ("/Global_Objects/Math/min",         "builtin"),
    ("/Global_Objects/Math/sin",         "builtin"),
    ("/Global_Objects/Math/cos",         "builtin"),
    ("/Global_Objects/Math/tan",         "builtin"),
    ("/Global_Objects/Math/log",         "builtin"),
    ("/Global_Objects/Math/exp",         "builtin"),
    ("/Global_Objects/Math/sign",        "builtin"),
    ("/Global_Objects/Math/trunc",       "builtin"),
    ("/Global_Objects/Math/cbrt",        "builtin"),
    ("/Global_Objects/Math/hypot",       "builtin"),
    ("/Global_Objects/Math/atan2",       "builtin"),
    ("/Global_Objects/Math/clz32",       "builtin"),
    ("/Global_Objects/Math/fround",      "builtin"),
    ("/Global_Objects/Math/imul",        "builtin"),
    ("/Global_Objects/Math/log1p",       "builtin"),
    ("/Global_Objects/Math/log2",        "builtin"),
    ("/Global_Objects/Math/log10",       "builtin"),
    ("/Global_Objects/Math/expm1",       "builtin"),
    # ── Built-in: Array ──────────────────────────────────────────
    ("/Global_Objects/Array/push",       "builtin"),
    ("/Global_Objects/Array/pop",        "builtin"),
    ("/Global_Objects/Array/shift",      "builtin"),
    ("/Global_Objects/Array/unshift",    "builtin"),
    ("/Global_Objects/Array/reverse",    "builtin"),
    ("/Global_Objects/Array/sort",       "builtin"),
    ("/Global_Objects/Array/indexOf",    "builtin"),
    ("/Global_Objects/Array/includes",   "builtin"),
    ("/Global_Objects/Array/join",       "builtin"),
    ("/Global_Objects/Array/slice",      "builtin"),
    ("/Global_Objects/Array/splice",     "builtin"),
    ("/Global_Objects/Array/forEach",    "builtin"),
    ("/Global_Objects/Array/map",        "builtin"),
    ("/Global_Objects/Array/reduce",     "builtin"),
    ("/Global_Objects/Array/filter",     "builtin"),
    ("/Global_Objects/Array/some",       "builtin"),
    ("/Global_Objects/Array/every",      "builtin"),
    ("/Global_Objects/Array/flat",       "builtin"),
    ("/Global_Objects/Array/flatMap",    "builtin"),
    ("/Global_Objects/Array/concat",     "builtin"),
    ("/Global_Objects/Array/find",       "builtin"),
    ("/Global_Objects/Array/findIndex",  "builtin"),
    ("/Global_Objects/Array/fill",       "builtin"),
    ("/Global_Objects/Array/at",         "builtin"),
    ("/Global_Objects/Array/isArray",    "builtin"),
    ("/Global_Objects/Array/from",       "builtin"),
    ("/Global_Objects/Array/of",         "builtin"),
    # ── Built-in: String ─────────────────────────────────────────
    ("/Global_Objects/String/charAt",     "builtin"),
    ("/Global_Objects/String/charCodeAt", "builtin"),
    ("/Global_Objects/String/codePointAt","builtin"),
    ("/Global_Objects/String/concat",     "builtin"),
    ("/Global_Objects/String/includes",   "builtin"),
    ("/Global_Objects/String/indexOf",    "builtin"),
    ("/Global_Objects/String/slice",      "builtin"),
    ("/Global_Objects/String/split",      "builtin"),
    ("/Global_Objects/String/startsWith", "builtin"),
    ("/Global_Objects/String/endsWith",  "builtin"),
    ("/Global_Objects/String/replace",    "builtin"),
    ("/Global_Objects/String/replaceAll","builtin"),
    ("/Global_Objects/String/repeat",     "builtin"),
    ("/Global_Objects/String/toUpperCase","builtin"),
    ("/Global_Objects/String/toLowerCase","builtin"),
    ("/Global_Objects/String/trim",       "builtin"),
    ("/Global_Objects/String/padStart",   "builtin"),
    ("/Global_Objects/String/padEnd",     "builtin"),
    ("/Global_Objects/String/substring",  "builtin"),
    ("/Global_Objects/String/match",      "builtin"),
    ("/Global_Objects/String/search",     "builtin"),
    ("/Global_Objects/String/at",         "builtin"),
    # ── Built-in: Map ────────────────────────────────────────────
    ("/Global_Objects/Map/Map",          "builtin"),
    ("/Global_Objects/Map/set",         "builtin"),
    ("/Global_Objects/Map/get",         "builtin"),
    ("/Global_Objects/Map/has",         "builtin"),
    ("/Global_Objects/Map/delete",      "builtin"),
    ("/Global_Objects/Map/size",        "builtin"),
    ("/Global_Objects/Map/forEach",     "builtin"),
    ("/Global_Objects/Map/keys",        "builtin"),
    ("/Global_Objects/Map/values",      "builtin"),
    ("/Global_Objects/Map/entries",     "builtin"),
    # ── Built-in: Set ────────────────────────────────────────────
    ("/Global_Objects/Set/Set",          "builtin"),
    ("/Global_Objects/Set/add",         "builtin"),
    ("/Global_Objects/Set/has",         "builtin"),
    ("/Global_Objects/Set/delete",      "builtin"),
    ("/Global_Objects/Set/size",        "builtin"),
    ("/Global_Objects/Set/forEach",     "builtin"),
    ("/Global_Objects/Set/keys",        "builtin"),
    ("/Global_Objects/Set/values",      "builtin"),
    # ── Built-in: Object ───────────────────────────────────────
    ("/Global_Objects/Object/keys",      "builtin"),
    ("/Global_Objects/Object/values",    "builtin"),
    ("/Global_Objects/Object/entries",   "builtin"),
    ("/Global_Objects/Object/assign",    "builtin"),
    ("/Global_Objects/Object/hasOwn",    "builtin"),
    ("/Global_Objects/Object/is",        "builtin"),
    ("/Global_Objects/Object/fromEntries","builtin"),
    # ── Built-in: Date ───────────────────────────────────────────
    ("/Global_Objects/Date/now",        "builtin"),
    ("/Global_Objects/Date/getFullYear","builtin"),
    ("/Global_Objects/Date/getMonth",    "builtin"),
    ("/Global_Objects/Date/getDate",     "builtin"),
    ("/Global_Objects/Date/getDay",      "builtin"),
    ("/Global_Objects/Date/getHours",    "builtin"),
    ("/Global_Objects/Date/getMinutes",  "builtin"),
    ("/Global_Objects/Date/getSeconds",  "builtin"),
    ("/Global_Objects/Date/getTime",    "builtin"),
    ("/Global_Objects/Date/toISOString","builtin"),
    # ── Built-in: JSON ───────────────────────────────────────────
    ("/Global_Objects/JSON/stringify",  "builtin"),
    ("/Global_Objects/JSON/parse",      "builtin"),
    # ── Built-in: Number ────────────────────────────────────────
    ("/Global_Objects/Number/isNaN",      "builtin"),
    ("/Global_Objects/Number/isFinite",   "builtin"),
    ("/Global_Objects/Number/isInteger",  "builtin"),
    ("/Global_Objects/Number/parseInt",   "builtin"),
    ("/Global_Objects/Number/parseFloat", "builtin"),
    ("/Global_Objects/Number/toFixed",     "builtin"),
    # ── Built-in: Global functions ─────────────────────────────
    ("/Global_Objects/isNaN",            "builtin"),
    ("/Global_Objects/isFinite",          "builtin"),
    ("/Global_Objects/parseInt",          "builtin"),
    ("/Global_Objects/parseFloat",        "builtin"),
    ("/Global_Objects/encodeURIComponent","builtin"),
    ("/Global_Objects/decodeURIComponent","builtin"),
    # ── Built-in: RegExp ───────────────────────────────────────
    ("/Global_Objects/RegExp/test",       "builtin"),
    ("/Global_Objects/RegExp/exec",      "builtin"),
]

URLS = [(f"{MDN}/en-US/docs/Web/JavaScript/Reference{s}", c) for s, c in SUPPORTED]

# ── Unsupported patterns (skip snippet if matched) ───────────────────
UNSUPPORTED = re.compile(
    "|".join([
        r"instanceof",
        r"function\s*\*(?!\s*[\w$])",
        r"\byield\b",
        r"for await",
        r"import\s*\(",
        r"new\.target",
        r"import\.meta",
        r"\.with\s*\(",
        r"toReversed|toSorted|toSpliced",
        r"groupBy",
        r"getOwnPropertySymbols",
        r"toUTCString",
        r"setTime",
        r"\.source\b|\.flags\b",
        r"\bclass\s+\w+\s+extends",
        r"\bdebugger\b",
        r"\bwith\s+",
        r"\barguments\b",
        r"Array\.prototype\.\w+\.call",
        # Browser APIs
        r"\bwindow\b",
        r"\bdocument\b",
        r"\bNodeFilter\b",
        r"\breadFile\b",
        r"\bXMLHttpRequest\b",
        r"\bfetch\b",
    ]),
    re.DOTALL,
)

# ── Manual operator test cases ────────────────────────────────────────
MANUAL_OPS = r"""
    // ── Arithmetic Operators ──────────────────────────────────────
    {
        let x = 1;
        console.log(2 + 3);
        console.log(10 - 4);
        console.log(3 * 7);
        console.log(17 / 5);
        console.log(17 % 5);
        console.log(2 ** 8);
        x = 5; console.log(++x);
        x = 5; console.log(x++);
    }
    // ── Comparison Operators ─────────────────────────────────────
    console.log(1 === 1);
    console.log(1 !== 2);
    console.log(3 > 2);
    console.log(3 >= 3);
    console.log(2 < 5);
    console.log(2 <= 2);
    // ── Logical Operators ───────────────────────────────────────
    console.log(true && true);
    console.log(true && false);
    console.log(false || true);
    console.log(false || false);
    console.log(null ?? "default");
    // ── Bitwise Operators ───────────────────────────────────────
    console.log(0b1100 & 0b1010);
    console.log(0b1100 | 0b1010);
    console.log(0b1100 ^ 0b1010);
    console.log(~0);
    console.log(8 << 2);
    console.log(32 >> 2);
    // ── Unary Operators ─────────────────────────────────────────
    console.log(-42);
    console.log(!true);
    console.log(!!1);
    // ── Conditional (Ternary) ─────────────────────────────────
    console.log(5 > 3 ? "yes" : "no");
    // ── typeof ───────────────────────────────────────────────────
    console.log(typeof 42);
    console.log(typeof "hi");
    console.log(typeof true);
    console.log(typeof undefined);
    // ── Template literals ──────────────────────────────────────
    console.log(`hello ${"world"}`);
    // ── Array / Object literals ─────────────────────────────────
    console.log([1, 2, 3].length);
    console.log({ a: 1, b: 2 }.a);
"""


# ── Helpers ──────────────────────────────────────────────────────────


def fetch(url: str) -> str:
    r = requests.get(url, timeout=30)
    r.raise_for_status()
    return r.text


def extract_snippets(html: str, max_per_page: int = 3) -> list[str]:
    soup = BeautifulSoup(html, "html.parser")
    out: list[str] = []
    for pre in soup.find_all("pre"):
        text = pre.get_text()
        if "console.log" not in text:
            continue
        # Skip: unsupported feature patterns (includes browser APIs now)
        if UNSUPPORTED.search(text):
            continue
        # Skip: MDN "Expected output" comment lines
        cleaned = "\n".join(
            ln for ln in text.splitlines()
            if not re.match(r"\s*//\s*Expected\s+output", ln, re.IGNORECASE)
        ).strip()
        # Skip: weird MDN examples (console.log as case value, etc.)
        if re.search(r"case\s+console\.", cleaned):
            continue
        # Skip: defines functions but never calls them (heuristic)
        defined = re.findall(r"\bfunction\s+(\w+)", cleaned)
        if defined:
            called = any(f"{name}(" in cleaned for name in defined)
            if defined and not called:
                continue
        # Skip: snippets that are error examples (have "// SyntaxError" in a comment)
        if re.search(r"SyntaxError", cleaned):
            # MDN marks error examples with "// SyntaxError: ..." comments.
            # Skip any snippet that has such a comment (anywhere in the line).
            has_syntaxerror_comment = False
            for ln in cleaned.splitlines():
                if re.search(r"//.*SyntaxError", ln):
                    has_syntaxerror_comment = True
                    break
            if has_syntaxerror_comment:
                continue
            # Also skip if we recognize specific error patterns
            if re.search(r"for\s*\(\s*(?:let|const|var)\s+\w+\s*=.*\s+in\b", cleaned):
                continue  # for-in with initializer
            if re.search(r"\btry\b[^{]*\S[^{]*$", cleaned, re.MULTILINE):
                continue  # try without block (syntax error)
        if len(cleaned) > 1500:
            continue
        # Skip: contains emoji/non-BMP characters (mangled by BS4)
        if has_emoji_or_bad_unicode(cleaned):
            continue
        if cleaned not in out:
            out.append(cleaned)
        if len(out) >= max_per_page:
            break
    return out


def has_emoji_or_bad_unicode(text: str) -> bool:
    """
    Return True if text contains emoji or non-BMP characters
    that might be mojibake when extracted from MDN.
    Also catches mojibake patterns (ISO-8859-1 chars in unexpected places).
    """
    for c in text:
        o = ord(c)
        # Non-BMP characters (emoji range, etc.)
        if o > 0xFFFF:
            return True
        # ISO-8859-1 control chars / special chars that suggest mojibake
        # (U+0080-U+00FF range, but allow common ones like copyright, etc.)
        # Actually, many MDN examples legitimately use non-ASCII.
        # Let's just check for the specific mojibake pattern.
    # Check for the specific mojibake pattern: U+0098, U+0083, etc.
    # These are control chars that shouldn't appear in JS source.
    if re.search(r"[\u0080-\u00A0\u00AD-\u00FF]", text):
        return True
    return False


def validate_js_file(path: Path, cat: str, bodies: list[str]) -> list[str]:
    """
    Run `node --check` on the generated JS file.
    If it has syntax errors, find and remove the problematic snippet.
    Returns the updated bodies list (with bad snippets removed).
    """
    while True:
        result = subprocess.run(
            ["node", "--check", str(path)],
            capture_output=True, text=True
        )
        if result.returncode == 0:
            return bodies  # No errors
        # Parse error to get line number
        # Error format: "file:///path:123\n        ^^^^"
        m = re.search(r":(\d+)\n", result.stderr)
        if not m:
            print(f"  WARNING: can't parse node --check error:")
            print(f"    {result.stderr[:300]}")
            return bodies
        error_line = int(m.group(1))
        print(f"  Syntax error at line {error_line}, finding problematic snippet...")
        # Regenerate the function text to find which snippet contains this line
        # Build the file content and count lines
        func_text = build_category_function(cat, bodies)
        func_lines = func_text.splitlines()
        # Find which body index contains error_line
        # (1-indexed lines in the function text)
        # The function text starts at line 1 of the file
        # (after the comment header)
        # Actually, let me just use a binary search approach:
        # Remove the snippet that contains this line.
        # Find by regenerating and checking line ranges.
        # Simpler: remove the last snippet (heuristic: errors often in last snippet)
        # Actually, let me do it properly:
        # Regenerate the file content and find which "    {" block contains error_line
        # This is complex. Use binary search:
        # Try removing snippets one by one from the end until the error goes away.
        if not bodies:
            print(f"  ERROR: no bodies left but file still has syntax error")
            return bodies
        # Find the snippet containing error_line
        # Generate the file content with line numbers
        header_lines = [
            "// Auto-generated from MDN JS Reference",
            f"// Category: {cat}",
            "",
        ]
        # Actually, let me use a simpler approach:
        # Comment out each snippet one-by-one and test.
        # But that's O(n) per error, and we might have multiple errors.
        # Use binary search: remove half the snippets, test, repeat.
        print(f"  Searching for problematic snippet (binary search)...")
        lo, hi = 0, len(bodies)
        bad_idx = -1
        # We know there's at least one bad snippet.
        # Binary search: test with subset of bodies
        test_bodies = list(bodies)
        while len(test_bodies) > 1:
            mid = len(test_bodies) // 2
            # Test with first half
            half_func = build_category_function(cat, test_bodies[:mid])
            half_path = path.with_suffix(".test.js")
            with open(half_path, "w", encoding="utf-8") as f:
                f.write(f"// Test file\n\n")
                f.write(half_func)
            r2 = subprocess.run(
                ["node", "--check", str(half_path)],
                capture_output=True, text=True
            )
            half_path.unlink()  # Clean up
            if r2.returncode != 0:
                # Error is in first half
                test_bodies = test_bodies[:mid]
            else:
                # Error is in second half
                test_bodies = test_bodies[mid:]
        bad_idx = bodies.index(test_bodies[0])
        print(f"  Removing snippet at index {bad_idx} (line {error_line})")
        bodies.pop(bad_idx)
        # Regenerate file
        with open(path, "w", encoding="utf-8") as f:
            f.write(f"// Auto-generated from MDN JS Reference\n")
            f.write(f"// Category: {cat}\n\n")
            f.write(build_category_function(cat, bodies))
        print(f"  Re-validation after removal...")
    return bodies  # unreachable, but...


def fix_switch_cases(snippet: str) -> str:
    """
    Wrap case clauses that have const/let declarations in {} blocks.
    In JS, `const` inside switch is hoisted to the enclosing block,
    so two case clauses with `const x` in the same switch conflict.
    Fix: add {} after case label if the case body declares variables.
    """
    lines = snippet.splitlines()
    result = []
    i = 0
    while i < len(lines):
        ln = lines[i]
        # Match "case ...:" or "default:"
        m = re.match(r"(\s*)(case\s+.+?|default)\s*:", ln)
        if m:
            indent = m.group(1)
            label = ln.rstrip()
            # Look ahead: does the case body already start with { ?
            j = i + 1
            while j < len(lines) and not lines[j].strip():
                j += 1
            if j < len(lines) and lines[j].strip() == '{':
                # Already wrapped
                result.append(ln)
                i = j  # will be incremented at end of loop
                continue
            # Collect case body lines until next case/default/} of switch
            body_start = j
            body_end = j
            has_decl = False
            while j < len(lines):
                nxt = lines[j]
                stripped = nxt.strip()
                if re.match(r"\s*(case\s+.+?|default)\s*:", nxt):
                    break
                if stripped == '}':
                    break
                if re.search(r"\b(const|let)\s+\w+", stripped):
                    has_decl = True
                body_end = j + 1
                j += 1
            if has_decl and body_start < len(lines):
                # Wrap body in {}
                result.append(f"{indent}{label} {{")
                for k in range(body_start, body_end):
                    result.append(lines[k])
                result.append(f"{indent}}}")
                i = body_end
                continue
            else:
                # No decl, keep as-is, advance to body_end
                result.append(ln)
                i = body_end if body_end > i + 1 else i + 1
                continue
        result.append(ln)
        i += 1
    return "\n".join(result)


def snippet_to_body(snippet: str) -> str:
    """
    Passthrough with fix: return the snippet as-is,
    but fix switch case scoping issues first.
    console.log calls are kept intact; run_all.js overrides
    console.log to capture output.
    """
    fixed = fix_switch_cases(snippet)
    return fixed


def build_category_function(cat: str, all_bodies: list[str], manual_code: str = "") -> str:
    """
    Build ONE exported function for a category.
    The function runs all snippets (which call console.log).
    Each snippet is wrapped in try-catch so runtime errors in one
    snippet don't prevent the rest from running.
    """
    func_name = f"test{cat.capitalize()}"
    parts = [
        f"export function {func_name}() {{",
    ]
    # Add manual code (for expression operators)
    if manual_code:
        for ln in manual_code.strip().splitlines():
            if ln.strip().startswith("//"):
                parts.append(f"    {ln}")
            elif ln.strip():
                parts.append(f"    {ln}")
    # Wrap each snippet body in a block scope + try-catch
    for body in all_bodies:
        parts.append(f"    {{")
        parts.append(f"        try {{")
        for ln in body.splitlines():
            if ln.strip():
                parts.append(f"            {ln}")
            else:
                parts.append("")
        parts.append(f"        }} catch (e) {{")
        parts.append(f"            console.error('[{func_name}] Error:', e.message);")
        parts.append(f"        }}")
        parts.append(f"    }}")
        parts.append("")
    parts.append(f"}}")
    return "\n".join(parts)


# ── Main ──────────────────────────────────────────────────────────


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry",  action="store_true", help="List URLs, no network")
    ap.add_argument("--test",  action="store_true", help="Fetch 4 pages, print generated JS")
    args = ap.parse_args()

    if args.dry:
        for url, cat in URLS:
            print(f"[{cat:12s}] {url}")
        return

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    # Collect converted bodies per category
    buckets: dict[str, list[str]] = {"statement": [], "builtin": []}
    # Expression category gets manual tests only (no scraping)
    expr_bodies: list[str] = []

    if args.test:
        for i, (url, cat) in enumerate(URLS[:4]):
            print(f"=== {url.split('/')[-1]} ===")
            try:
                html = fetch(url)
                snippets = extract_snippets(html, max_per_page=2)
                for j, s in enumerate(snippets):
                    body = snippet_to_body(s)
                    print(f"  -- snippet {j} --")
                    print("    " + body.replace("\n", "\n    "))
                    print()
                    buckets[cat].append(body)
            except Exception as e:
                print(f"ERROR: {e}")
        # Print the combined function
        for cat in ["statement", "builtin"]:
            if buckets[cat]:
                print(f"=== Generated test{cat.capitalize()}() ===")
                print(build_category_function(cat, buckets[cat]))
        return

    # ── Full run ───────────────────────────────────────────────────
    for i, (url, cat) in enumerate(URLS):
        print(f"[{i+1:3d}/{len(URLS)}] {url.split('/')[-1]:30s} ... ", end="", flush=True)
        try:
            html = fetch(url)
            snippets = extract_snippets(html)
            if not snippets:
                print("no snippets")
                continue
            for s in snippets:
                buckets[cat].append(snippet_to_body(s))
            print(f"{len(snippets)} snippet(s)")
        except Exception as e:
            print(f"ERROR: {e}")
        time.sleep(0.2)

    # ── Write the 3 JS files ───────────────────────────────────────
    # 1. test_expressions.js  (manual operator tests only)
    path_expr = OUTPUT_DIR / "test_expressions.js"
    with open(path_expr, "w", encoding="utf-8") as f:
        f.write("// Auto-generated from MDN JS Reference\n")
        f.write("// Category: expressions (operators)\n")
        f.write("// NOTE: MDN operator pages are conceptual guides;\n")
        f.write("// operator test cases are added manually below.\n\n")
        f.write(build_category_function("expressions", expr_bodies, MANUAL_OPS))
    # Validate and auto-fix
    expr_bodies = validate_js_file(path_expr, "expressions", expr_bodies)
    print(f"  → {path_expr.name}  ({len(expr_bodies)} snippets)")

    # 2. test_statements.js
    path_stmt = OUTPUT_DIR / "test_statements.js"
    with open(path_stmt, "w", encoding="utf-8") as f:
        f.write("// Auto-generated from MDN JS Reference\n")
        f.write("// Category: statements\n\n")
        if buckets["statement"]:
            f.write(build_category_function("statements", buckets["statement"]))
        else:
            f.write("export function testStatements() { return []; }\n")
    # Validate and auto-fix
    buckets["statement"] = validate_js_file(path_stmt, "statements", buckets["statement"])
    print(f"  → {path_stmt.name}  ({len(buckets['statement'])} snippets)")

    # 3. test_builtins.js
    path_blt = OUTPUT_DIR / "test_builtins.js"
    with open(path_blt, "w", encoding="utf-8") as f:
        f.write("// Auto-generated from MDN JS Reference\n")
        f.write("// Category: built-in objects\n\n")
        if buckets["builtin"]:
            f.write(build_category_function("builtins", buckets["builtin"]))
        else:
            f.write("export function testBuiltins() { return []; }\n")
    # Validate and auto-fix
    buckets["builtin"] = validate_js_file(path_blt, "builtins", buckets["builtin"])
    print(f"  → {path_blt.name}  ({len(buckets['builtin'])} snippets)")

    # ── Write run_all.js (Node.js runner) ───────────────────────
    runner = OUTPUT_DIR / "run_all.js"
    with open(runner, "w", encoding="utf-8") as f:
        f.write("// Run all 3 test functions and capture console output.\n")
        f.write("// Usage:  node run_all.js\n\n")
        f.write("const _captured = [];\n")
        f.write("const _origLog   = console.log;\n")
        f.write("const _origError = console.error;\n")
        f.write("const _origWarn  = console.warn;\n")
        f.write("console.log = function(...args) {\n")
        f.write("    _captured.push(args.length === 1 ? args[0] : args);\n")
        f.write("};\n")
        f.write("console.error = function(...args) {\n")
        f.write("    _captured.push('[error] ' + (args.length === 1 ? args[0] : args.join(' ')));\n")
        f.write("};\n")
        f.write("console.warn = function(...args) {\n")
        f.write("    _captured.push('[warn] ' + (args.length === 1 ? args[0] : args.join(' ')));\n")
        f.write("};\n\n")
        f.write('import { testExpressions } from "./test_expressions.js";\n')
        f.write('import { testStatements } from "./test_statements.js";\n')
        f.write('import { testBuiltins }   from "./test_builtins.js";\n')
        f.write("\n")
        f.write("const results = {};\n\n")
        f.write("_captured.length = 0;\n")
        f.write("testExpressions();\n")
        f.write("results.expressions = [..._captured];\n\n")
        f.write("_captured.length = 0;\n")
        f.write("testStatements();\n")
        f.write("results.statements = [..._captured];\n\n")
        f.write("_captured.length = 0;\n")
        f.write("testBuiltins();\n")
        f.write("results.builtins = [..._captured];\n\n")
        f.write("console.log   = _origLog;\n")
        f.write("console.error = _origError;\n")
        f.write("console.warn  = _origWarn;\n")
        f.write('console.log(JSON.stringify(results, null, 2));\n')
    print(f"\nRunner: {runner}")
    print(f"Run:  node run_all.js  > ../../expected_output.json")


if __name__ == "__main__":
    main()
