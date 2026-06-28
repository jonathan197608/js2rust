#!/usr/bin/env python3
"""
scrape_mdn_v3.py  —  从 MDN JS 参考页抓取测试用例。

改进：
  - 正确处理代码缩进（去公共缩进后再统一缩进）
  - 处理 await（用 async IIFE 包装）
  - 使用 CommonJS module.exports（兼容 Node.js 直接运行）
  - 更严格的语法过滤
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

try:
    import requests
    from bs4 import BeautifulSoup
except ImportError:
    print("Missing deps:  pip install requests beautifulsoup4")
    sys.exit(1)

SCRIPT_DIR = Path(__file__).resolve().parent
OUTPUT_DIR = SCRIPT_DIR.parent / "examples" / "mdn-test-project" / "js_src"
MDN_REF    = "https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference"

UNSUPPORTED = [
    r"function\s*\*\s*\(",
    r"async\s+function\s*\*\s*\(",
    r"\byield\b",
    r"for\s+await\s*\(\s*",
    r"\bimport\s*\(",
    r"\bimport\.meta\b",
    r"\bnew\.target\b",
    r"\?\.",
    r"#\w+",
    r"\bextends\s+\w+",
    r"\bwith\s*\(",
    r"\bdebugger\b",
    r"\barguments\b",
    r"\{\s*\w+\s*,\s*\w+\s*\}\s*=",
    r"\[\s*\w+\s*,\s*\w+\s*\]\s*=",
    r"\bWeakMap\b",
    r"\bWeakSet\b",
    r"\bProxy\b",
    r"\bReflect\b",
    r"\bIntl\.",
    r"\bAtomics\b",
    r"\bSharedArrayBuffer\b",
    r"\bFinalizationRegistry\b",
    r"\bWeakRef\b",
    r"\.toReversed\s*\(",
    r"\.toSorted\s*\(",
    r"\.toSpliced\s*\(",
    r"\.with\s*\(",
    r"\.groupBy\s*\(",
    r"\.getOwnPropertySymbols\s*\(",
    r"\bwindow\b",
    r"\bdocument\b",
    r"\bXMLHttpRequest\b",
    r"\bfetch\s*\(",
]
UNSUPPORTED_RE = re.compile("|".join(UNSUPPORTED))

SKIP_SLUGS = {
    "Global_Objects", "Statements", "Operators", "Functions",
    "Classes", "Lexical_grammar", "Template_literals",
    "Inheritance_and_the_prototype_chain", "Strict_mode",
    "Memory_management", "Equality_comparisons_and_sameness",
    "Closures", "Object-oriented_JavaScript", "Object_prototype",
}


def classify(url: str) -> str:
    if "/Statements/" in url or "/statements/" in url:
        return "statements"
    if "/Operators/" in url or "/operators/" in url:
        return "expressions"
    return "builtins"


def fetch(url: str, timeout: int = 30) -> str:
    r = requests.get(url, timeout=timeout,
                     headers={"User-Agent": "js2rust-mdn-scraper/3.0"})
    r.raise_for_status()
    return r.text


def discover_urls() -> list[tuple[str, str]]:
    print(f"Discovering URLs from {MDN_REF} ...")
    html = fetch(MDN_REF)
    soup = BeautifulSoup(html, "html.parser")
    results: list[tuple[str, str]] = []
    seen: set[str] = set()
    for a in soup.find_all("a", href=True):
        href = a["href"].split("#")[0].rstrip("/")
        if "/docs/Web/JavaScript/Reference" not in href:
            continue
        if "/en-US/" not in href:
            continue
        full = href if href.startswith("http") else "https://developer.mozilla.org" + href
        slug = full.split("/")[-1]
        if slug in SKIP_SLUGS:
            continue
        if full in seen:
            continue
        seen.add(full)
        cat = classify(full)
        results.append((full, cat))
    print(f"  Found {len(results)} pages")
    return results


def clean_block(raw: str) -> str | None:
    lines = raw.splitlines()
    out = []
    for ln in lines:
        if re.search(r"//\s*Expected\s+output", ln, re.IGNORECASE):
            continue
        stripped = ln.strip()
        if stripped.startswith("export "):
            kept = re.sub(r"^export\s+(default\s+)?", "", stripped)
            indent = ln[:len(ln) - len(ln.lstrip())]
            out.append(indent + kept)
            continue
        if stripped.startswith("import "):
            continue
        out.append(ln)
    code = "\n".join(out).strip()
    if not code:
        return None
    for c in code:
        o = ord(c)
        if o > 0xFFFF or (0x0080 <= o <= 0x009F):
            return None
    non_empty = [l for l in code.splitlines() if l.strip() and not l.strip().startswith("//")]
    if not non_empty:
        return None
    return code


def dedent(code: str) -> str:
    lines = code.splitlines()
    indents = []
    for ln in lines:
        s = ln.lstrip()
        if s:
            indents.append(len(ln) - len(s))
    if not indents:
        return code
    mi = min(indents)
    return "\n".join(ln[mi:] if ln.strip() else "" for ln in lines)


def validate_syntax(js_code: str) -> tuple[bool, str]:
    tmp = OUTPUT_DIR / "._tmp_check.js"
    try:
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(js_code)
            f.write("\n")
        r = subprocess.run(["node", "--check", str(tmp)],
                          capture_output=True, text=True, timeout=10)
        return (r.returncode == 0, r.stderr[:300] if r.stderr else "")
    except Exception as e:
        return (False, str(e))
    finally:
        if tmp.exists():
            tmp.unlink()


def fragment_needs_async(code: str) -> bool:
    if not re.search(r"\bawait\b", code):
        return False
    if re.search(r"async\s+function|async\s*\(", code):
        return False
    return True


def extract_fragments(html: str) -> list[str]:
    soup = BeautifulSoup(html, "html.parser")
    raw: list[str] = []
    for pre in soup.find_all("pre"):
        text = pre.get_text()
        if not text.strip():
            continue
        if re.search(r"<[a-z][^>]*>", text, re.IGNORECASE):
            continue
        cleaned = clean_block(text)
        if cleaned:
            raw.append(cleaned)
    if not raw:
        return []
    valid: list[str] = []
    for block in raw:
        ok, _ = validate_syntax(block)
        if ok:
            valid.append(block)
    if not valid:
        return []
    merged = "\n\n".join(valid)
    if UNSUPPORTED_RE.search(merged):
        return []
    return [dedent(v) for v in valid]


def wrap_test_function(cat: str, fragments: list[str]) -> str:
    fn = f"test{cat.capitalize()}"
    lines = [f"function {fn}() {{"]
    for i, frag in enumerate(fragments):
        lines.append(f"    // ---- fragment {i} ----")
        lines.append("    try {{")
        needs_async = fragment_needs_async(frag)
        if needs_async:
            lines.append("        (async () => {{")
            for ln in frag.splitlines():
                if ln.strip():
                    lines.append("            " + ln)
                else:
                    lines.append("")
            lines.append("        }})();")
        else:
            for ln in frag.splitlines():
                if ln.strip():
                    lines.append("        " + ln)
                else:
                    lines.append("")
        lines.append("    }} catch (e) {{")
        lines.append(f'        console.error(`[{fn}] fragment {i} error: ${{e.message}}`);')
        lines.append("    }}")
        lines.append("")
    lines.append("}")
    lines.append(f"module.exports = {{ {fn} }};")
    return "\n".join(lines)


def generate_runner(out_dir: Path):
    runner = out_dir / "run_all.js"
    content = [
        "// Run all MDN test functions.",
        "// Usage:  node run_all.js  > expected_output.json",
        "",
        "const _captured = [];",
        "const _orig = console.log;",
        "console.log = function(...args) {",
        "    _captured.push(args.length === 1 ? String(args[0]) : args.map(String).join(' '));",
        "};",
        "",
        "const results = {};",
        "",
        "try {",
        "    const { testExpressions } = require('./test_expressions.js');",
        "    _captured.length = 0;",
        "    testExpressions();",
        "    results.expressions = [..._captured];",
        "} catch(e) { results.expressions = ['[FATAL] ' + e.message]; }",
        "",
        "try {",
        "    const { testStatements } = require('./test_statements.js');",
        "    _captured.length = 0;",
        "    testStatements();",
        "    results.statements = [..._captured];",
        "} catch(e) { results.statements = ['[FATAL] ' + e.message]; }",
        "",
        "try {",
        "    const { testBuiltins } = require('./test_builtins.js');",
        "    _captured.length = 0;",
        "    testBuiltins();",
        "    results.builtins = [..._captured];",
        "} catch(e) { results.builtins = ['[FATAL] ' + e.message]; }",
        "",
        "console.log = _orig;",
        "console.log(JSON.stringify(results, null, 2));",
    ]
    with open(runner, "w", encoding="utf-8") as f:
        f.write("\n".join(content))
    print(f"  ✓ {runner.name}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--max-pages", type=int, default=0, help="Max pages (0=all)")
    args = ap.parse_args()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    url_cat_pairs = discover_urls()
    if args.max_pages > 0:
        url_cat_pairs = url_cat_pairs[:args.max_pages]

    print(f"Scraping {len(url_cat_pairs)} pages ...\n")
    buckets: dict[str, list[str]] = {"statements": [], "expressions": [], "builtins": []}

    for i, (url, cat) in enumerate(url_cat_pairs):
        slug = url.split("/")[-1]
        print(f"  [{i+1}/{len(url_cat_pairs)}] {cat:15s} {slug:40s}", end="", flush=True)
        try:
            html = fetch(url)
            fragments = extract_fragments(html)
            if not fragments:
                print("  no valid JS")
                continue
            merged = "\n\n".join(fragments)
            ok, err = validate_syntax(merged)
            if not ok:
                print(f"  merged syntax error: {err[:80]}")
                continue
            buckets[cat].extend(fragments)
            print(f"  ✓ ({len(fragments)} fragments)")
        except Exception as e:
            print(f"  ERROR: {e}")
        time.sleep(0.15)

    print(f"\nGenerating test files ...")
    for cat in ["statements", "expressions", "builtins"]:
        fragments = buckets[cat]
        if not fragments:
            print(f"  {cat}: no fragments, skipping")
            continue
        fpath = OUTPUT_DIR / f"test_{cat}.js"
        code = wrap_test_function(cat, fragments)
        with open(fpath, "w", encoding="utf-8") as f:
            f.write(f"// Auto-generated from MDN JS Reference\n")
            f.write(f"// Category: {cat}\n")
            f.write(f"// Fragments: {len(fragments)}\n")
            f.write(f"// Generated: {time.strftime('%Y-%m-%d')}\n\n")
            f.write(code)
        ok, err = validate_syntax(open(fpath, encoding="utf-8").read())
        if not ok:
            print(f"  ✗ {fpath.name} syntax error:\n{err}")
        else:
            print(f"  ✓ {fpath.name} ({len(fragments)} fragments)")

    generate_runner(OUTPUT_DIR)
    print(f"\nDone. Files in {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
