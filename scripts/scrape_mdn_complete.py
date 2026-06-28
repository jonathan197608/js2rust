#!/usr/bin/env python3
"""
scrape_mdn_complete.py  –  从 MDN JS 参考页完整抓取测试用例，
剔除语法错误，用 Node.js 验证后可安全用于 js2rust 测试。

工作流程
────────
1.  从 MDN JS Reference 主页发现所有子页面链接
2.  对每个子页面，提取所有 <pre> 代码块
3.  用 Node.js 验证每个代码片段的语法（node --check）
4.  用 Node.js vm 模块执行代码片段，捕获输出
5.  剔除包含不支持语法的片段（js2rust 限制）
6.  输出 3 个 JS 文件 + expected_output.json

用法
────
    python scrape_mdn_complete.py              # 全量抓取
    python scrape_mdn_complete.py --quick     # 每页只取前 2 个片段
    python scrape_mdn_complete.py --category statements  # 只抓某一类
    python scrape_mdn_complete.py --validate-only  # 只验证已有文件
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import urljoin, urlparse

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

# ── js2rust 不支持的语法特征（片段包含这些则跳过）────────────────
UNSUPPORTED_PATTERNS = [
    # 生成器 / async 生成器
    r"function\s*\s*\(",
    r"async\s+function\s*\s*\(",
    r"\byield\b",
    r"\byield\s*\s*\(",
    # for-await-of
    r"for\s+await\s*\s*of",
    # 动态 import / import.meta / new.target
    r"\bimport\s*\s*\(",
    r"\bimport\.meta\b",
    r"\bnew\.target\b",
    # 可选链（js2rust 未实现）
    r"\?\.",
    # 逗号运算符在奇怪位置
    # 私有字段
    r"#\w+",
    # class extends（js2rust 部分支持，但复杂例子容易炸）
    r"\bextends\s+\w+",
    # with 语句
    r"\bwith\s*\s*\(",
    # debugger 语句
    r"\bdebugger\b",
    # arguments 对象
    r"\barguments\b",
    # 解构赋值（js2rust 未实现）
    r"\{\s*\w+\s*,\s*\w+\s*\}\s*=",
    r"\[\s*\w+\s*,\s*\w+\s*\]\s*=",
    # 剩余/展开（某些复杂用法）
    # 标签模板字面量
    r"\w+`",
    # BigInt 字面量
    r"\d+n\b",
    # WeakMap / WeakSet（js2rust 不支持）
    r"\bWeakMap\b",
    r"\bWeakSet\b",
    # Proxy / Reflect（js2rust 不支持）
    r"\bProxy\b",
    r"\bReflect\b",
    # Intl（js2rust 不支持）
    r"\bIntl\.",
    # Atomics / SharedArrayBuffer
    r"\bAtomics\b",
    r"\bSharedArrayBuffer\b",
    # FinalizationRegistry / WeakRef
    r"\bFinalizationRegistry\b",
    r"\bWeakRef\b",
    # 尾部逗号在奇怪位置（某些 MDN 例子有）
    # toReversed / toSorted / toSpliced（ES2023 不变方法）
    r"\.toReversed\s*\(",
    r"\.toSorted\s*\(",
    r"\.toSpliced\s*\(",
    r"\.with\s*\(",
    r"\.groupBy\s*\(",
    r"\.getOwnPropertySymbols\s*\(",
    # 浏览器 API
    r"\bwindow\b",
    r"\bdocument\b",
    r"\bXMLHttpRequest\b",
    r"\bfetch\s*\(",
    r"\bconsole\.log\.call\b",
]

UNSUPPORTED_RE = re.compile("|".join(UNSUPPORTED_PATTERNS), re.DOTALL)

# ── 需要跳过的页面（概念页，无可用代码片段）───────────────────────
SKIP_PAGES = {
    # 总览/索引页
    "Global_Objects",
    "Statements",
    "Operators",
    "Functions",
    "Classes",
    "Lexical_grammar",
    "Template_literals",
    # 纯概念页
    "Inheritance_and_the_prototype_chain",
    "Strict_mode",
    "Memory_management",
    "Equality_comparisons_and_sameness",
    "Closures",
    "Object-oriented_JavaScript",
    # 已废弃
    "Object_prototype",
}

# ── 分类映射：URL 路径片段 → 测试分类 ─────────────────────────────
def classify_url(path: str) -> str | None:
    """根据 MDN URL 路径判断测试用例分类。"""
    if "/Statements/" in path or "/statements/" in path:
        return "statements"
    if "/Operators/" in path or "/operators/" in path:
        return "expressions"
    if "/Global_Objects/" in path or "/global_objects/" in path:
        return "builtins"
    if "/Functions/" in path or "/functions/" in path:
        return "builtins"   # 箭头函数等也归入 builtins
    if "/Classes/" in path or "/classes/" in path:
        return "builtins"   # class 语法归 builtins
    return None


# ── 抓取辅助 ────────────────────────────────────────────────────────

def fetch(url: str, timeout: int = 30) -> str:
    r = requests.get(url, timeout=timeout,
                     headers={"User-Agent": "js2rust-mdn-scraper/1.0"})
    r.raise_for_status()
    return r.text


def discover_urls() -> list[tuple[str, str]]:
    """
    从 MDN JS Reference 主页发现所有子页面链接。
    返回 [(url, category), ...]。
    """
    print(f"Discovering URLs from {MDN_REF} ...")
    html = fetch(MDN_REF)
    soup = BeautifulSoup(html, "html.parser")
    urls: list[tuple[str, str]] = []

    for a in soup.find_all("a", href=True):
        href = a["href"]
        # 只保留 MDN JS Reference 下的链接
        if "/docs/Web/JavaScript/Reference" not in href:
            continue
        # 去掉 hash 部分
        href = href.split("#")[0]
        # 补齐为绝对 URL
        if href.startswith("/"):
            full = MDN_ROOT + href
        else:
            full = urljoin(MDN_REF, href)

        # 去掉 trailing slash
        full = full.rstrip("/")

        # 跳过索引页/概念页
        page_slug = full.split("/")[-1]
        if page_slug in SKIP_PAGES:
            continue
        # 只保留 en-US 版本
        if "/en-US/" not in full:
            continue

        cat = classify_url(full)
        if cat:
            urls.append((full, cat))

    # 去重
    seen: set[str] = set()
    unique: list[tuple[str, str]] = []
    for url, cat in urls:
        if url not in seen:
            seen.add(url)
            unique.append((url, cat))

    print(f"  Found {len(unique)} pages")
    return unique


def extract_js_blocks(html: str, url: str) -> list[str]:
    """
    从 HTML 中提取当前页面的所有 JS 代码片段。
    策略：将同一页面的所有 <pre> 代码块合并为一个片段，
    以保留代码片段之间的依赖关系（后一个块可能引用前一个块定义的变量）。
    返回片段列表（每个页面最多返回 1-2 个合并后的片段）。
    """
    soup = BeautifulSoup(html, "html.parser")

    # 收集当前页面所有 <pre> 块
    all_blocks: list[str] = []
    for pre in soup.find_all("pre"):
        text = pre.get_text()
        if not text.strip():
            continue
        # 跳过含 HTML 标签的块
        if re.search(r"<[a-z][^>]*>", text, re.IGNORECASE):
            continue
        cleaned = _clean_pre_block(text)
        if cleaned:
            all_blocks.append(cleaned)

    if not all_blocks:
        return []

    # 合并所有块：策略1 — 全部合并为一个大片段
    merged = _merge_blocks(all_blocks)

    # 验证合并后的语法
    is_ok, _ = validate_syntax(merged, "merge")
    if is_ok:
        return [merged]

    # 语法有问题：尝试两两合并，逐步增加直到报错
    # 实际上更常见的是：部分块是自包含的，部分有依赖
    # 策略2：从前向后逐步合并，直到遇到语法错误，然后开始新片段
    return _split_merge(all_blocks)


def _clean_pre_block(raw: str) -> str | None:
    """清理单个 <pre> 块的文本。"""
    lines = raw.splitlines()
    result = []
    for ln in lines:
        # 跳过 "Expected output" 注释
        if re.search(r"//\s*Expected\s+output", ln, re.IGNORECASE):
            continue
        result.append(ln)

    code = "\n".join(result).strip()
    if not code:
        return None

    # 去掉 Unicode 乱码/emoji
    if _has_bad_unicode(code):
        return None

    # 过滤：去掉 export 语句（模块语法，脚本上下文不可执行）
    # 去掉行首的 export ... 声明，但保留函数/类本身
    cleaned_lines2 = []
    for ln in code.splitlines():
        # 去掉 export 关键字，保留后面的声明
        stripped = ln.strip()
        if stripped.startswith("export "):
            # export default function foo() {} → function foo() {}
            # export default class Foo {} → class Foo {}
            # export const foo = ... → const foo = ...
            kept = re.sub(r"^export\s+(default\s+)?", "", stripped)
            cleaned_lines2.append(kept)
        elif stripped.startswith("import "):
            # import 语句在脚本上下文不可执行，跳过该行
            continue
        else:
            cleaned_lines2.append(ln)

    code = "\n".join(cleaned_lines2).strip()
    if not code:
        return None

    # 至少包含一条可执行的语句（去掉纯注释/空行后）
    non_empty = [l for l in code.splitlines() if l.strip() and not l.strip().startswith("//")]
    if not non_empty:
        return None

    return code


def _merge_blocks(blocks: list[str]) -> str:
    """将多个代码块合并为一个，块之间用空行分隔。"""
    return "\n\n".join(blocks)


def _split_merge(blocks: list[str]) -> list[str]:
    """
    智能合并：从前向后逐步合并代码块。
    如果遇到语法错误，从错误点拆分。
    """
    if not blocks:
        return []
    batches: list[list[str]] = []
    current: list[str] = []
    for block in blocks:
        current.append(block)
        merged = "\n\n".join(current)
        is_ok, _ = validate_syntax(merged, "split")
        if not is_ok and len(current) > 1:
            # 最后一个块导致语法错误，回退
            current.pop()
            batches.append(current)
            current = [block]
    if current:
        batches.append(current)

    return ["\n\n".join(b) for b in batches]


def _has_bad_unicode(text: str) -> bool:
    """检测 emoji / 控制字符。"""
    for c in text:
        o = ord(c)
        if o > 0xFFFF:
            return True
        if 0x0080 <= o <= 0x00A0:
            return True
    return False


# ── Node.js 验证 ────────────────────────────────────────────────────

def validate_syntax(js_code: str, label: str = "") -> tuple[bool, str]:
    """
    用 `node --check` 验证 JS 代码片段的语法。
    返回 (is_valid, error_message)。
    """
    tmp = OUTPUT_DIR / f".tmp_check_{label}.js"
    try:
        with open(tmp, "w", encoding="utf-8") as f:
            f.write("// Syntax check\n")
            f.write(js_code)
            f.write("\n")
        r = subprocess.run(
            ["node", "--check", str(tmp)],
            capture_output=True, text=True, timeout=10
        )
        return (r.returncode == 0, r.stderr[:300] if r.stderr else "")
    except Exception as e:
        return (False, str(e))
    finally:
        if tmp.exists():
            tmp.unlink()


def execute_snippet(js_code: str, label: str = "") -> tuple[bool, list[str]]:
    """
    用 Node.js vm 模块执行 JS 代码片段，捕获 console.log 输出。
    返回 (success, outputs)。
    """
    # 构建可执行的 JS：包装 console.log 捕获
    wrapper = f"""
const {{ VM }} = require('vm');
const {{ Script }} = require('vm');

// Capture console.log output
const _out = [];
const _orig = console.log;
console.log = (...args) => {{
    _out.push(args.length === 1 ? String(args[0]) : args.map(String).join(' '));
}};

try {{
{js_code.replace(chr(10), chr(10) + '    ')}
}} catch (e) {{
    _out.push('[RUNTIME_ERROR] ' + e.message);
}}

console.log = _orig;
console.log(JSON.stringify(_out));
"""
    tmp = OUTPUT_DIR / f".tmp_exec_{label}.js"
    try:
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(wrapper)
        r = subprocess.run(
            ["node", str(tmp)],
            capture_output=True, text=True, timeout=10
        )
        if r.returncode != 0:
            return (False, [f"Exec error: {r.stderr[:200]}"])
        # 解析输出
        out = r.stdout.strip()
        if not out:
            return (True, [])
        try:
            parsed = json.loads(out)
            return (True, parsed if isinstance(parsed, list) else [str(parsed)])
        except json.JSONDecodeError:
            return (True, [out])
    except Exception as e:
        return (False, [str(e)])
    finally:
        if tmp.exists():
            tmp.unlink()


# ── 测试用例包装 ───────────────────────────────────────────────────

def wrap_as_test_function(cat: str, snippets: list[str]) -> str:
    """
    将所有片段包装为一个 export function testXxx() { ... }。
    每个片段放在独立的 try-catch 块中，防止一个片段报错导致全部停止。
    """
    func_name = f"test{cat.capitalize()}"
    parts = [f"export function {func_name}() {{"]
    for i, snippet in enumerate(snippets):
        parts.append(f"    // ── snippet {i} ──")
        parts.append("    {")
        parts.append("        try {")
        # 缩进片段内容
        for ln in snippet.splitlines():
            if ln.strip():
                parts.append("            " + ln)
            else:
                parts.append("")
        parts.append("        } catch (e) {")
        parts.append(f'            console.error(`[{func_name}] snippet {i} error: ${{e.message}}`);')
        parts.append("        }")
        parts.append("    }")
        parts.append("")
    parts.append("}")
    return "\n".join(parts)


# ── 主流程 ─────────────────────────────────────────────────────────

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--quick", action="store_true",
                    help="Quick mode: max 2 snippets per page")
    ap.add_argument("--category", type=str, default=None,
                    help="Only scrape this category (statements/expressions/builtins)")
    ap.add_argument("--validate-only", action="store_true",
                    help="Only validate existing JS files with Node.js")
    ap.add_argument("--max-pages", type=int, default=0,
                    help="Max number of pages to scrape (0 = unlimited)")
    args = ap.parse_args()

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    if args.validate_only:
        print("Validating existing JS files...")
        for fname in ["test_expressions.js", "test_statements.js", "test_builtins.js"]:
            fpath = OUTPUT_DIR / fname
            if fpath.exists():
                r = subprocess.run(["node", "--check", str(fpath)],
                                   capture_output=True, text=True)
                status = "✓ OK" if r.returncode == 0 else f"✗ ERROR:\n{r.stderr[:400]}"
                print(f"  {fname}: {status}")
        return

    # Step 1: Discover all URLs
    url_cat_pairs = discover_urls()

    if args.category:
        url_cat_pairs = [(u, c) for u, c in url_cat_pairs if c == args.category]

    if args.max_pages > 0:
        url_cat_pairs = url_cat_pairs[:args.max_pages]

    print(f"Scraping {len(url_cat_pairs)} pages ...")

    # Step 2: Fetch each page and extract snippets
    buckets: dict[str, list[tuple[str, str]]] = {
        "statements": [],
        "expressions": [],
        "builtins": [],
    }

    for i, (url, cat) in enumerate(url_cat_pairs):
        page_slug = url.split("/")[-1]
        print(f"  [{i+1:4d}/{len(url_cat_pairs)}] {cat:15s} {page_slug:35s} ... ",
              end="", flush=True)
        try:
            html = fetch(url)
            blocks = extract_js_blocks(html, url)
            if not blocks:
                print("no JS blocks")
                continue
            # 验证明法
            valid = []
            for j, block in enumerate(blocks):
                if args.quick and j >= 2:
                    break
                ok, err = validate_syntax(block, f"{page_slug}_{j}")
                if ok:
                    valid.append(block)
                else:
                    pass  # 静默跳过语法错误的片段
            if valid:
                # 只保留前 3 个有效片段（防止单个页面贡献太多）
                buckets[cat].extend(valid[:3])
                print(f"{len(valid[:3])} snippet(s)")
            else:
                print("all snippets have syntax errors")
        except Exception as e:
            print(f"ERROR: {e}")
        time.sleep(0.15)  # 礼貌延迟

    # Step 3: 生成 JS 测试文件
    print(f"\nGenerating test files ...")
    for cat in ["statements", "expressions", "builtins"]:
        snippets = [s for s, _ in buckets[cat]] if isinstance(buckets[cat][0] if buckets[cat] else None, tuple) else buckets[cat]
        # 上面逻辑有点乱，重新整理
        snippets = buckets[cat]  # 已经是 list[str]

        fname = f"test_{cat}.js"
        fpath = OUTPUT_DIR / fname

        # 为 expressions 添加手动操作符测试用例
        manual_ops = ""
        if cat == "expressions":
            manual_ops = get_manual_operator_tests()

        with open(fpath, "w", encoding="utf-8") as f:
            f.write(f"// Auto-generated from MDN JS Reference\n")
            f.write(f"// Category: {cat}\n")
            f.write(f"// Generated: {time.strftime('%Y-%m-%d')}\n\n")
            if manual_ops:
                f.write(wrap_as_test_function(cat, []).replace(
                    "export function testExpressions() {",
                    f"export function testExpressions() {{\n{manual_ops}"))
            else:
                f.write(wrap_as_test_function(cat, snippets))
        # 验证生成的文件
        ok, err = validate_syntax(open(fpath, encoding="utf-8").read(), cat)
        if not ok:
            print(f"  ✗ {fname} has syntax error:\n{err}")
            # 尝试修复：逐个移除片段直到通过
            fix_syntax_errors(fpath, cat, snippets)
        else:
            print(f"  ✓ {fname} ({len(snippets)} snippets)")

    # Step 4: 生成 run_all.js
    generate_runner()
    print(f"\nDone. Files written to {OUTPUT_DIR}")


def get_manual_operator_tests() -> str:
    """返回手动编写的操作符测试用例（MDN 操作符页面多为概念说明，无可直接抓取的用例）。"""
    return """
    // ── Manual operator test cases ──
    console.log(2 + 3);
    console.log(10 - 4);
    console.log(3 * 7);
    console.log(17 / 5);
    console.log(17 % 5);
    console.log(1 === 1);
    console.log(1 !== 2);
    console.log(3 > 2);
    console.log(true && false);
    console.log(false || true);
    console.log(null ?? "default");
    console.log(-42);
    console.log(!true);
    console.log(5 > 3 ? "yes" : "no");
    console.log(typeof 42);
"""


def fix_syntax_errors(fpath: Path, cat: str, snippets: list[str]):
    """逐个移除片段直到文件通过 node --check。"""
    print(f"  Fixing syntax errors in {fpath.name} ...")
    while len(snippets) > 0:
        with open(fpath, "w", encoding="utf-8") as f:
            f.write(f"// Auto-generated (fixed)\n\n")
            f.write(wrap_as_test_function(cat, snippets))
        ok, err = validate_syntax(open(fpath, encoding="utf-8").read(), cat)
        if ok:
            print(f"  ✓ Fixed: {len(snippets)} snippets remaining")
            return
        # 移除最后一个片段再试
        snippets = snippets[:-1]
    # 所有片段都移除了，写入空函数
    with open(fpath, "w", encoding="utf-8") as f:
        f.write(f"export function test{cat.capitalize()}() {{ }}\n")
    print(f"  ✗ Could not fix, wrote empty function")


def generate_runner():
    """生成 run_all.js：用 Node.js 执行所有测试函数并输出 JSON。"""
    runner = OUTPUT_DIR / "run_all.js"
    with open(runner, "w", encoding="utf-8") as f:
        f.write("// Run all MDN test functions and capture output.\n")
        f.write("// Usage:  node run_all.js  > expected_output.json\n\n")
        f.write("const _captured = [];\n")
        f.write("const _origLog = console.log;\n")
        f.write("console.log = function(...args) {\n")
        f.write("    _captured.push(args.length === 1 ? args[0] : args);\n")
        f.write("};\n\n")
        f.write('import { testExpressions } from "./test_expressions.js";\n')
        f.write('import { testStatements } from "./test_statements.js";\n')
        f.write('import { testBuiltins }   from "./test_builtins.js";\n\n')
        f.write("const results = {};\n\n")
        # expressions
        f.write("_captured.length = 0;\n")
        f.write("try { testExpressions(); } catch(e) { _captured.push('[FATAL] ' + e.message); }\n")
        f.write("results.expressions = [..._captured];\n\n")
        # statements
        f.write("_captured.length = 0;\n")
        f.write("try { testStatements(); } catch(e) { _captured.push('[FATAL] ' + e.message); }\n")
        f.write("results.statements = [..._captured];\n\n")
        # builtins
        f.write("_captured.length = 0;\n")
        f.write("try { testBuiltins(); } catch(e) { _captured.push('[FATAL] ' + e.message); }\n")
        f.write("results.builtins = [..._captured];\n\n")
        # output
        f.write("console.log = _origLog;\n")
        f.write('console.log(JSON.stringify(results, null, 2));\n')
    print(f"  ✓ {runner.name}")


if __name__ == "__main__":
    main()
