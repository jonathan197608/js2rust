#!/usr/bin/env python3
"""
scrape_mdn_v2.py  —  从 MDN JS 参考页完整抓取测试用例，
合并同一页面的多个代码块，用 Node.js 验证语法，
输出可直接用于 js2rust 测试的 JS 文件。

关键改进：
  - 同一页面的所有 <pre> 代码块合并为一个片段（保留依赖关系）
  - 自动剔除 export/import 模块语句
  - 用 `node --check` 验证语法
  - 输出 3 个 test_*.js 文件 + run_all.js

用法：
    python scripts/scrape_mdn_v2.py              # 全量抓取
    python scripts/scrape_mdn_v2.py --quick      # 每页最多 2 个片段
    python scripts/scrape_mdn_v2.py --category statements
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

# ── 路径 ───────────────────────────────────────────────────────────
SCRIPT_DIR = Path(__file__).resolve().parent
OUTPUT_DIR = SCRIPT_DIR.parent / "examples" / "mdn-test-project" / "js_src"
MDN_ROOT   = "https://developer.mozilla.org"
MDN_REF    = MDN_ROOT + "/en-US/docs/Web/JavaScript/Reference"

# ── js2rust 不支持的语法（包含这些的片段跳过）───────────────────
UNSUPPORTED = [
    r"function\s*\*\s*\(",     # generator
    r"async\s+function\s*\*\s*\(",  # async generator
    r"\byield\b",
    r"for\s+await\s*\(\s*",   # for-await-of
    r"\bimport\s*\(",            # dynamic import
    r"\bimport\.meta\b",
    r"\bnew\.target\b",
    r"\?\.",                     # optional chaining
    r"#\w+",                    # private field
    r"\bextends\s+\w+",        # class extends
    r"\bwith\s*\(",             # with statement
    r"\bdebugger\b",
    r"\barguments\b",
    r"\{\s*\w+\s*,\s*\w+\s*\}\s*=",  # destructuring
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
    r"\.\btoReversed\s*\(",
    r"\.\btoSorted\s*\(",
    r"\.\btoSpliced\s*\(",
    r"\.\bwith\s*\(",
    r"\.groupBy\s*\(",
    r"\.getOwnPropertySymbols\s*\(",
    r"\bwindow\b",
    r"\bdocument\b",
    r"\bXMLHttpRequest\b",
    r"\bfetch\s*\(",
    r"\bconsole\.log\.call\b",
]
UNSUPPORTED_RE = re.compile("|".join(UNSUPPORTED))

# ── 跳过无代码片段的概念页 ────────────────────────────────────────
SKIP_SLUGS = {
    "Global_Objects", "Statements", "Operators", "Functions",
    "Classes", "Lexical_grammar", "Template_literals",
    "Inheritance_and_the_prototype_chain", "Strict_mode",
    "Memory_management", "Equality_comparisons_and_sameness",
    "Closures", "Object-oriented_JavaScript", "Object_prototype",
}


def classify(url: str) -> str | None:
    if "/Statements/" in url or "/statements/" in url:
        return "statements"
    if "/Operators/" in url or "/operators/" in url:
        return "expressions"
    if "/Global_Objects/" in url:
        return "builtins"
    return "builtins"  # Functions/Classes 也归 builtins


def fetch(url: str, timeout: int = 30) -> str:
    r = requests.get(url, timeout=timeout,
                     headers={"User-Agent": "js2rust-mdn-scraper/2.0"})
    r.raise_for_status()
    return r.text


def discover_urls() -> list[tuple[str, str]]:
    """从 MDN JS Reference 主页发现所有子页面。"""
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
        full = href if href.startswith("http") else MDN_ROOT + href
        slug = full.split("/")[-1]
        if slug in SKIP_SLUGS:
            continue
        if full in seen:
            continue
        seen.add(full)
        cat = classify(full)
        if cat:
            results.append((full, cat))

    print(f"  Found {len(results)} pages")
    return results


def clean_block(raw: str) -> str | None:
    """
    清理单个 <pre> 块的文本。
    去掉 export/import 关键字，去掉 Expected output 注释，
    返回清理后的代码或 None。
    """
    lines = raw.splitlines()
    out_lines = []
    for ln in lines:
        stripped = ln.strip()
        # 跳过 Expected output 注释行
        if re.search(r"//\s*Expected\s+output", stripped, re.IGNORECASE):
            continue
        # 去掉 export 关键字（保留后面的声明）
        if stripped.startswith("export "):
            kept = re.sub(r"^export\s+(default\s+)?", "", stripped)
            # 把整行替换
            indent = ln[:len(ln) - len(ln.lstrip())]
            out_lines.append(indent + kept)
            continue
        # 跳过 import 语句
        if stripped.startswith("import "):
            continue
        out_lines.append(ln)

    code = "\n".join(out_lines).strip()
    if not code:
        return None

    # 去掉 emoji / 控制字符
    for c in code:
        o = ord(c)
        if o > 0xFFFF or (0x0080 <= o <= 0x009F):
            return None

    # 跳过纯注释/空行
    non_comment = [l for l in code.splitlines()
                   if l.strip() and not l.strip().startswith("//")]
    if not non_comment:
        return None

    return code


def extract_page_fragments(html: str) -> list[str]:
    """
    提取当前页面的所有 JS 代码块，逐个验证语法，
    只返回通过 node --check 的块（剔除故意展示错误的示例）。
    返回有效片段列表（每个元素是一个完整的可独立执行的代码块）。
    """
    soup = BeautifulSoup(html, "html.parser")
    raw_blocks: list[str] = []

    for pre in soup.find_all("pre"):
        text = pre.get_text()
        if not text.strip():
            continue
        # 跳过含 HTML 标签的块（如 <div> 示例）
        if re.search(r"<[a-z][^>]*>", text, re.IGNORECASE):
            continue
        cleaned = clean_block(text)
        if cleaned:
            raw_blocks.append(cleaned)

    if not raw_blocks:
        return []

    # 逐个验证语法，只保留有效的
    valid: list[str] = []
    for block in raw_blocks:
        ok, _ = validate_syntax(block)
        if ok:
            valid.append(block)

    if not valid:
        return []

    # 检查是否包含 js2rust 不支持的语法（在合并后的完整片段中检查）
    merged = "\n\n".join(valid)
    if UNSUPPORTED_RE.search(merged):
        return []

    return valid


def validate_syntax(js_code: str) -> tuple[bool, str]:
    """用 node --check 验证 JS 语法。返回 (ok, error_msg)。"""
    tmp = OUTPUT_DIR / "._tmp_check.js"
    try:
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(js_code)
            f.write("\n")
        r = subprocess.run(
            ["node", "--check", str(tmp)],
            capture_output=True, text=True, timeout=10
        )
        return (r.returncode == 0, r.stderr[:400] if r.stderr else "")
    except Exception as e:
        return (False, str(e))
    finally:
        if tmp.exists():
            tmp.unlink()


def _fragment_needs_async(code: str) -> bool:
    """检查片段是否需要 async 包装（包含顶层 await）。"""
    if not re.search(r"\bawait\b", code):
        return False
    if re.search(r"async\s+function|async\s*\(", code):
        return False
    return True


def wrap_test_function(cat: str, fragments: list[str]) -> str:
    """
    将多个片段包装为一个 testXxx() 函数，使用 CommonJS 格式。
    包含 await 的片段会用 (async () => { ... })() 包装。
    """
    func_name = f"test{cat.capitalize()}"
    parts = [f"function {func_name}() {{"]
    for i, frag in enumerate(fragments):
        parts.append(f"    // ---- fragment {i} ----")
        parts.append("    try {")
        if _fragment_needs_async(frag):
            # 用 async IIFE 包装
            parts.append("        (async () => {")
            for ln in frag.splitlines():
                if ln.strip():
                    parts.append("            " + ln)
                else:
                    parts.append("")
            parts.append("        })();")
        else:
            for ln in frag.splitlines():
                if ln.strip():
                    parts.append("            " + ln)
                else:
                    parts.append("")
        parts.append("    } catch (e) {")
        parts.append(f'        console.error(`[{func_name}] fragment {i} error: ${{e.message}}`);')
        parts.append("    }")
        parts.append("")
    parts.append("}")
    parts.append("")
    parts.append(f"module.exports = {{ {func_name} }};")
    return "\\n".join(parts)


def generate_runner(out_dir: Path):
    """生成 run_all.js：依次调用 3 个测试函数，输出 JSON。"""
    runner = out_dir / "run_all.js"
    lines = [
        "// Run all MDN test functions and capture output.",
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
        "// expressions",
        "try {",
        "    const { testExpressions } = require('./test_expressions.js');",
        "    _captured.length = 0;",
        "    testExpressions();",
        "    results.expressions = [..._captured];",
        "} catch(e) { results.expressions = ['[FATAL] ' + e.message]; }",
        "",
        "// statements",
        "try {",
        "    const { testStatements } = require('./test_statements.js');",
        "    _captured.length = 0;",
        "    testStatements();",
        "    results.statements = [..._captured];",
        "} catch(e) { results.statements = ['[FATAL] ' + e.message]; }",
        "",
        "// builtins",
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
        f.write("\n".join(lines))
    print(f"  ✓ {runner.name}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--quick", action="store_true",
                    help="Quick mode: max 1 fragment per page")
    ap.add_argument("--category", type=str, default=None,
                    help="Only scrape: statements / expressions / builtins")
    ap.add_argument("--max-pages", type=int, default=0,
                    help="Max pages to scrape (0 = all)")
    args = ap.parse_args()

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Step 1: Discover URLs
    url_cat_pairs = discover_urls()
    if args.category:
        url_cat_pairs = [(u, c) for u, c in url_cat_pairs if c == args.category]
    if args.max_pages > 0:
        url_cat_pairs = url_cat_pairs[:args.max_pages]

    print(f"Scraping {len(url_cat_pairs)} pages ...\n")

    # Step 2: Fetch + extract + validate
    buckets: dict[str, list[str]] = {"statements": [], "expressions": [], "builtins": []}

    for i, (url, cat) in enumerate(url_cat_pairs):
        slug = url.split("/")[-1]
        print(f"  [{i+1}/{len(url_cat_pairs)}] {cat:15s} {slug:40s}",
              end="", flush=True)

        try:
            html = fetch(url)
            fragments = extract_page_fragments(html)
            if not fragments:
                print("  no valid JS")
                continue

            # quick 模式：每页最多保留 2 个片段
            if args.quick and len(fragments) > 2:
                fragments = fragments[:2]

            # 验证合并后的语法（双重检查）
            merged = "\n\n".join(fragments)
            ok, err = validate_syntax(merged)
            if not ok:
                print(f"  merged syntax error: {err[:80]}")
                continue

            buckets[cat].extend(fragments)
            print(f"  ✓ ({len(fragments)} fragments, {len(merged)} chars)")

        except Exception as e:
            print(f"  ERROR: {e}")
        time.sleep(0.2)

    # Step 3: 生成 JS 文件
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

        # 验证生成的文件
        ok, err = validate_syntax(open(fpath, encoding="utf-8").read())
        if not ok:
            print(f"  ✗ {fpath.name} syntax error:\n{err}")
        else:
            print(f"  ✓ {fpath.name} ({len(fragments)} fragments)")

    # Step 4: 生成 run_all.js
    generate_runner(OUTPUT_DIR)
    print(f"\nDone. Files in {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
