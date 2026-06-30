#!/usr/bin/env python3
"""
Auto-fix scraped JS fragments for js2zig compatibility.

Fixes two categories of Zig compilation errors:
1. Undeclared identifiers — insert `var name = <init>;` at fragment start
2. Unused local constants — append `_ = name;` at fragment end

Also handles:
3. Cross-fragment nested function references
4. Other known patterns that cause Zig errors
"""

import re
import sys
from pathlib import Path
from collections import defaultdict

PROJECT_DIR = Path(__file__).resolve().parent.parent / "examples" / "mdn-test-project"
JS_SRC = PROJECT_DIR / "js_src"

# ── Known JS globals / builtins that do NOT need declaration ──
JS_GLOBALS = {
    # Core objects
    "console", "Math", "Date", "String", "Number", "Boolean", "Symbol", "BigInt",
    "Object", "Array", "Map", "Set", "RegExp", "Error", "JSON", "Promise",
    "Proxy", "Reflect", "Function", "DataView", "ArrayBuffer",
    # Global functions
    "isNaN", "isFinite", "parseInt", "parseFloat",
    "encodeURI", "decodeURI", "encodeURIComponent", "decodeURIComponent",
    "eval", "setTimeout", "setInterval", "clearTimeout", "clearInterval",
    # Global values
    "Infinity", "NaN", "undefined", "arguments", "toString", "valueOf",
    "hasOwnProperty", "isPrototypeOf", "propertyIsEnumerable",
    "toLocaleString", "constructor", "prototype",
    "length", "name", "call", "apply", "bind",
    # Common method names used as identifiers in fragments
    "charAt", "charCodeAt", "codePointAt", "concat", "includes",
    "indexOf", "lastIndexOf", "match", "matchAll", "padEnd", "padStart",
    "repeat", "replace", "replaceAll", "search", "slice", "split",
    "startsWith", "endsWith", "substring", "toLowerCase", "toUpperCase",
    "trim", "trimStart", "trimEnd", "localeCompare", "normalize",
    "forEach", "map", "filter", "reduce", "reduceRight", "some", "every",
    "find", "findIndex", "findLast", "findLastIndex", "flat", "flatMap",
    "push", "pop", "shift", "unshift", "splice", "sort", "reverse",
    "keys", "values", "entries", "has", "get", "set", "delete", "clear",
    "then", "catch", "finally",
    "toString", "valueOf",
    # JS builtin properties
    "PI", "E", "LN10", "LN2", "LOG10E", "LOG2E", "SQRT1_2", "SQRT2",
    "MAX_VALUE", "MIN_VALUE", "EPSILON", "MAX_SAFE_INTEGER", "MIN_SAFE_INTEGER",
    "NEGATIVE_INFINITY", "POSITIVE_INFINITY",
    "fromCharCode", "fromCodePoint", "raw",
    "now", "parse", "UTC",
    # Known DOM-like objects used in MDN fragments
    "alert", "document", "window", "fetch",
    "XMLHttpRequest", "navigator",
    # TypedArray constructors
    "Int8Array", "Uint8Array", "Uint8ClampedArray",
    "Int16Array", "Uint16Array", "Int32Array", "Uint32Array",
    "Float32Array", "Float64Array",
    "BigInt64Array", "BigUint64Array",
    # Error types
    "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError",
    "AggregateError", "InternalError",
    # Web APIs that appear in MDN fragments
    "WeakMap", "WeakSet", "SharedArrayBuffer",
    "FinalizationRegistry", "WeakRef",
    "Intl", "Atomics",
    # Common builtin methods (for call detection)
    "module",
}

# JS reserved words / keywords (never need declaration)
JS_KEYWORDS = {
    "if", "else", "for", "while", "do", "switch", "case", "break", "continue",
    "return", "throw", "try", "catch", "finally", "function", "class", "new",
    "typeof", "instanceof", "void", "delete", "in", "of", "this", "super",
    "true", "false", "null", "let", "const", "var", "export", "import",
    "async", "await", "yield", "debugger", "with", "default", "extends", "static",
    "from", "as", "get", "set",
    "use",  # "use strict"
}

# Smart initializers for known undeclared variables
SMART_INIT = {
    # Generic placeholder variables used in MDN examples
    "x": "1",
    "y": "2",
    "z": "3",
    "a": "1",
    "b": "2",
    "c": "3",
    "i": "0",
    "j": "0",
    "n": "0",
    # Boolean conditions
    "bCondition1": "true",
    "bCondition2": "false",
    "bCondition3": "true",
    # Function calls
    "doSomething": "function doSomething() { return 0; }",
    # Objects with method calls
    "checkbox": "{}",
    "_testJsAny": "{}",
    # Generic values
    "invalid": "42",
    "array": "[1, 2, 3, 4, 5]",
    "value": "0",
    "string": '"0"',
    "encodedURI": '"https://example.com"',
    "uri": '"https://example.com"',
    "expression": "0",
    "columns": "0",
    "table": '"default"',
    "el": "{}",
    "done": "false",
    "globalThis": "{}",
    "a1": "false",      # Boolean logical test variables
    "a2": "false",
    "a3": "false",
    "a4": "false",
    "a5": "false",
    "a6": "false",
    "a7": "false",
    "a8": "false",
    "a9": "false",
    "encodeRFC5987ValueChars": "function encodeRFC5987ValueChars(s) { return s; }",
    # Color enum
    "RED": "0",
    "GREEN": "1",
    "BLUE": "2",
}


def is_builtin(name: str) -> bool:
    """Check if name is a JS builtin or keyword."""
    return name in JS_GLOBALS or name in JS_KEYWORDS


def strip_regex_literals(code: str) -> str:
    """Strip JavaScript regex literals from code.

    JS regex literals take the form /pattern/flags and can be at the start of
    an expression, after =, (, [, !, etc. We replace them with a placeholder
    to avoid their content being misidentified as identifiers.
    """
    # Match regex literals: /.../ followed by optional flags
    # Heuristic: regex appears after operators, assignments, or at statement start
    result = []
    i = 0
    while i < len(code):
        if code[i] == "/" and i < len(code) - 1:
            # Check if this is a regex literal (not division and not comment)
            is_regex = False
            if i == 0:
                is_regex = True
            elif i > 0:
                # Find previous non-whitespace character
                p = i - 1
                while p >= 0 and code[p] in " \t\n\r":
                    p -= 1
                prev_ch = code[p] if p >= 0 else ""
                # Regex follows: = ( ) [ { , ; : ! & | ? ~ return typeof void delete
                if prev_ch in "=([{,;:!&|?~":
                    is_regex = True
                elif code[max(0, p - 5) : p + 1].strip().endswith("return"):
                    is_regex = True
                elif code[max(0, p - 6) : p + 1].strip().endswith("typeof"):
                    is_regex = True
                elif code[max(0, p - 4) : p + 1].strip().endswith("void"):
                    is_regex = True
                elif code[max(0, p - 5) : p + 1].strip().endswith("delete"):
                    is_regex = True
                elif code[max(0, p - 3) : p + 1].strip().endswith("case"):
                    is_regex = True
                elif code[max(0, p - 2) : p + 1].strip().endswith("in"):
                    is_regex = True
                elif code[max(0, p - 9) : p + 1].strip().endswith("instanceof"):
                    is_regex = True

            if is_regex:
                # Find the closing /
                j = i + 1
                while j < len(code):
                    if code[j] == "\\":
                        j += 2  # skip escaped char
                        continue
                    if code[j] == "/":
                        # Found closing slash — skip flags too
                        j += 1
                        while j < len(code) and code[j].isalpha():
                            j += 1
                        break
                    if code[j] == "\n":
                        # Unterminated regex — don't treat as regex
                        j = i + 1
                        break
                    j += 1
                else:
                    j = i + 1

                if j > i + 1:
                    # Replace regex literal with placeholder
                    result.append("/_REGEX_/")
                    i = j
                    continue

        result.append(code[i])
        i += 1

    return "".join(result)


def strip_strings_and_comments(code: str) -> str:
    """Strip string literals, regex literals, and comments from code."""
    # Remove template literals
    code = re.sub(r"`[^`]*`", "``", code)
    # Remove double-quoted strings
    code = re.sub(r'"[^"]*"', '""', code)
    # Remove single-quoted strings
    code = re.sub(r"'[^']*'", "''", code)
    # Remove single-line comments
    code = re.sub(r"//[^\n]*", "", code)
    # Remove block comments
    code = re.sub(r"/\*.*?\*/", "", code, flags=re.DOTALL)
    # Remove regex literals (must be after comment removal)
    code = strip_regex_literals(code)
    return code


def find_outer_declarations(content: str) -> set[str]:
    """Find function declarations in the outer function scope (before first fragment).

    These are function declarations hoisted by manual fixes or originally placed
    at the top of the part file function body. They are available to all fragments.
    """
    # Find the outer function body: from "function test_xxx() {" to first fragment
    m = re.search(r"function\s+\w+\s*\([^)]*\)\s*\{", content)
    if not m:
        return set()
    body_start = m.end()

    # Find the first fragment marker
    first_frag = content.find("// ---- fragment", body_start)
    if first_frag < 0:
        return set()

    outer_code = content[body_start:first_frag]

    # Extract function declarations from outer scope
    decls = set()
    for m in re.finditer(r"\bfunction\s+(\w+)\s*\(", outer_code):
        decls.add(m.group(1))

    return decls


def extract_fragments(content: str) -> list[dict]:
    """Parse a JS part file into fragments.

    Returns list of dicts with keys:
        num: fragment number
        start: byte offset in content
        end: byte offset in content (exclusive)
        text: full fragment text (including try/catch wrapper)
        inner: inner code (inside try {{ ... }})
    """
    fragments = []
    pattern = re.compile(r"// ---- fragment (\d+) ----\n")
    matches = list(pattern.finditer(content))

    for i, m in enumerate(matches):
        frag_start = m.start()
        frag_num = int(m.group(1))

        if i + 1 < len(matches):
            frag_end = matches[i + 1].start()
        else:
            # Last fragment — find closing brace of outer function
            frag_end = len(content)

        frag_text = content[frag_start:frag_end]

        # Extract inner code (inside try {{ ... }})
        # The pattern is: try {{ ... }} catch (e) {{ ... }}
        inner_m = re.search(r"try\s*\{\{(.*?)\}\}\s*catch\s*\(", frag_text, re.DOTALL)
        if inner_m:
            inner_code = inner_m.group(1)
        else:
            inner_code = frag_text

        fragments.append(
            {
                "num": frag_num,
                "start": frag_start,
                "end": frag_end,
                "text": frag_text,
                "inner": inner_code,
            }
        )

    return fragments


# ── Declaration detection ──


def find_declarations(code: str) -> set[str]:
    """Find all variable/function/param declarations in code."""
    declared = set()

    # var/let/const simple declarations
    for m in re.finditer(r"\b(?:var|let|const)\s+(\w+)\s*[=;,\n]", code):
        declared.add(m.group(1))

    # const {a, b} = ...  destructuring
    for m in re.finditer(r"\b(?:var|let|const)\s*\{([^}]+)\}\s*=", code):
        inner = m.group(1)
        for name_m in re.finditer(r"(\w+)\s*[,:}]", inner):
            declared.add(name_m.group(1))
        # Single element: {a}
        for name_m in re.finditer(r"\b(\w+)\s*\}", inner):
            declared.add(name_m.group(1))

    # Array destructuring: const [a, b] = ...
    for m in re.finditer(r"\b(?:var|let|const)\s*\[([^\]]+)\]\s*=", code):
        for name_m in re.finditer(r"\b(\w+)\b", m.group(1)):
            name = name_m.group(1)
            if not is_builtin(name):
                declared.add(name)

    # Function declarations: function name(...)
    for m in re.finditer(r"\bfunction\s+(\w+)\s*\(([^)]*)\)", code):
        declared.add(m.group(1))
        # Also add parameter names
        params = m.group(2)
        for param_m in re.finditer(r"\b(\w+)\b", params):
            pname = param_m.group(1)
            if not is_builtin(pname):
                declared.add(pname)

    # Arrow function parameters: (a, b) => or a =>
    for m in re.finditer(r"\(\s*(\w+(?:\s*,\s*\w+)*)\s*\)\s*=>", code):
        for name in re.findall(r"\w+", m.group(1)):
            if not is_builtin(name):
                declared.add(name)
    # Single-param arrow: x =>
    for m in re.finditer(r"(?<![a-zA-Z0-9_$.])(\w+)\s*=>", code):
        name = m.group(1)
        if not is_builtin(name):
            declared.add(name)

    # Catch parameters: catch (e) or catch (err)
    for m in re.finditer(r"\bcatch\s*\(\s*(\w+)", code):
        declared.add(m.group(1))

    # for-of loop variable: for (let x of ...)
    for m in re.finditer(r"\bfor\s*\(\s*(?:let|const|var)\s+(\w+)\s", code):
        declared.add(m.group(1))

    # for-in loop variable
    # Already caught by let/const/var patterns

    # Class declarations
    for m in re.finditer(r"\bclass\s+(\w+)", code):
        declared.add(m.group(1))

    return declared


# ── Reference detection ──


def find_references(code: str) -> set[str]:
    """Find all identifier references (usage, not declaration) in code."""
    cleaned = strip_strings_and_comments(code)

    refs = set()
    # Find all identifiers
    for m in re.finditer(r"\b([a-zA-Z_$][a-zA-Z0-9_$]*)\b", cleaned):
        name = m.group(1)
        pos = m.start()

        if is_builtin(name):
            continue

        # Skip if preceded by '.' (property access: obj.method)
        if pos > 0 and cleaned[pos - 1] == ".":
            continue

        # Skip if followed by ':' (property key in object literal, or label)
        after_pos = m.end()
        # Skip any whitespace between identifier and ':'
        while after_pos < len(cleaned) and cleaned[after_pos] in " \t":
            after_pos += 1
        if after_pos < len(cleaned) and cleaned[after_pos] == ":":
            continue

        # Skip if it's a function/class/var/let/const declaration name
        before = cleaned[max(0, pos - 20) : pos]
        if re.search(
            r"\b(?:function|class|var|let|const|catch)\s+$", before
        ):
            continue

        # Skip label references (break label; / continue label;)
        if re.search(r"\b(?:break|continue)\s+$", before):
            continue

        # Skip string literals content. Only flag when there's a real unmatched
        # opening quote (not just empty "" left by string stripping).
        # Use count parity to detect unmatched quotes.
        dq_count = before.count('"')
        sq_count = before.count("'")
        if dq_count % 2 == 1 or sq_count % 2 == 1:
            continue

        refs.add(name)

    return refs


# ── Analysis ──


def find_arrow_params(code: str) -> set[str]:
    """Find all arrow function parameter names."""
    params = set()
    # Multi-param: (a, b) => or (a) =>
    for m in re.finditer(r"\(\s*(\w+(?:\s*,\s*\w+)*)\s*\)\s*=>", code):
        for name_m in re.finditer(r"\b(\w+)\b", m.group(1)):
            params.add(name_m.group(1))
    # Single-param without parens: x =>
    for m in re.finditer(r"(?<![a-zA-Z0-9_$.])(\w+)\s*=>", code):
        params.add(m.group(1))
    return params


def analyze_fragment(frag: dict, outer_decls: set[str] = None) -> tuple[set[str], set[str]]:
    """Analyze a fragment for undeclared and unused variables.

    Returns (undeclared_set, unused_set).
    """
    if outer_decls is None:
        outer_decls = set()
    code = frag["inner"]

    declared = find_declarations(code) | outer_decls
    refs = find_references(code)

    # Undeclared: referenced but not declared and not a global
    undeclared = refs - declared

    # Filter out identifiers that look like regex literal artifacts:
    # - Single lowercase letters (regex flags: g, i, m, s, u, v, y)
    # - Hex-like patterns: u2028, u0065, x01
    # - Short alphanumeric combos from regex alternation groups
    # - Known false positives from regex Unicode property escapes
    regex_artifacts = {
        "ab", "bc", "ca", "ba", "abc", "aeiou", "gv", "gi", "iu", "gu",
        "nb", "sic", "gif", "jpe", "png", "webp", "avif",
        # Regex Unicode property escapes that leak through
        "Lowercase_Letter", "ID_Start", "ID_Continue",
        "RGI_Emoji_Flag_Sequence", "Emoji", "Emoji_Modifier",
        "Emoji_Modifier_Base", "Emoji_Component", "Emoji_Presentation",
        "Extended_Pictographic", "Script", "Latin", "Greek", "Common",
        "Letter", "Mark", "Number", "Punctuation", "Symbol", "Separator",
        "Other", "Control", "Format", "Surrogate", "Private_Use", "Unassigned",
        "ASCII", "Alphabetic", "Math", "Uppercase", "Lowercase",
        "White_Space", "Join_Control", "Logical_Order_Exception",
        "Noncharacter_Code_Point", "Default_Ignorable_Code_Point",
        # MDN article titles/SVG names that appear in comments
        "Using", "Warning", "Temporal", "MDN_Macro",
        # Regex placeholder from our stripping
        "_REGEX_",
        # Known builtin types that shouldn't be used as variables
        "integer", "float", "void",
        # Test framework names
        "assert", "expect", "describe", "it",
    }
    # Single-letter identifiers like x, y, z are legitimate JS variables in
    # MDN expression fragments (e.g., x >> y, x ** y). The regex stripping
    # already removes actual regex literals, so these are real variables.
    # Only filter genuine regex artifacts (Unicode escapes, hex patterns).
    filtered_undeclared = set()
    for name in sorted(undeclared):
        if len(name) == 1 and name.isupper():
            continue  # regex \P, \S, \D, \W etc.
        if re.match(r"^u[0-9a-fA-F]{4}$", name):
            continue
        if re.match(r"^x[0-9a-fA-F]{2}$", name):
            continue
        # Filter only long number suffixes (4+ digits) — short ones like a1, b2
        # are legitimate JS variable names (e.g., a1 = true && true).
        if re.match(r"^[a-z][0-9]{4,}$", name):
            continue
        if name in regex_artifacts:
            continue
        if name == "_":
            continue
        filtered_undeclared.add(name)
    undeclared = filtered_undeclared

    # Unused: declared LOCALLY but not referenced (exclude outer decls)
    local_declared = find_declarations(code)
    unused = local_declared - refs
    unused.discard("e")  # catch parameter that Zig renames to 'err'
    unused.discard("err")

    # Exclude arrow function parameters from unused — they are scoped
    # to the arrow function body, not the fragment. _ = param; at the
    # fragment level would reference an out-of-scope variable.
    arrow_params = find_arrow_params(code)
    unused -= arrow_params

    return undeclared, unused


# ── Fix application ──


def apply_fixes_to_file(filepath: Path) -> bool:
    """Fix undeclared and unused variables in a single part file.

    Uses a 2-pass approach:
      Pass 1: Insert `var name = init;` for undeclared variables.
      Pass 2: Insert `_ = name;` for variables that are now locally declared
              but not referenced (catches regex artifacts, Before/After
              placeholders, property keys, etc. that pass 1 inserted).
    """
    content = filepath.read_text(encoding="utf-8")
    original_content = content
    modified = False

    for pass_num in (1, 2):
        outer_decls = find_outer_declarations(content)
        fragments = extract_fragments(content)
        if not fragments:
            break

        # Collect all fixes: (position, insertion_text, reason)
        fixes = []

        for frag in fragments:
            undeclared, unused = analyze_fragment(frag, outer_decls)

            if not undeclared and not unused:
                continue

            # Pass 1: Insert var declarations for undeclared
            # Pass 2: Only insert suppressors for unused (post-pass-1 state)
            if pass_num == 1 and undeclared:
                # Find the position right after "try {{"
                try_match = re.search(
                    r"try\s*\{\{", frag["text"]
                )
                if try_match:
                    insert_pos = frag["start"] + try_match.end()
                    nl_pos = content.find("\n", insert_pos)
                    if nl_pos >= 0:
                        insert_pos = nl_pos + 1
                    else:
                        insert_pos = frag["start"] + try_match.end()

                    decl_lines = []
                    for var_name in sorted(undeclared):
                        if var_name in SMART_INIT:
                            init = SMART_INIT[var_name]
                            if init.startswith("function "):
                                decl_lines.append(f"        {init}")
                            else:
                                decl_lines.append(f"        var {var_name} = {init};")
                        else:
                            decl_lines.append(f"        var {var_name} = 0;")

                    if decl_lines:
                        fix_code = "\n".join(decl_lines) + "\n"
                        fixes.append(
                            (insert_pos, fix_code, f"pass1 undeclared: {sorted(undeclared)}")
                        )

            # Pass 2: Insert _ = name; suppressors for unused
            if pass_num == 2 and unused:
                catch_pos = frag["text"].rfind("}} catch")
                if catch_pos >= 0:
                    insert_pos = frag["start"] + catch_pos
                    ref_lines = []
                    for var_name in sorted(unused):
                        ref_lines.append(f"        _ = {var_name};")
                    if ref_lines:
                        fix_code = "\n".join(ref_lines) + "\n"
                        fixes.append(
                            (insert_pos, fix_code, f"pass2 unused: {sorted(unused)}")
                        )

        if not fixes:
            if pass_num == 1:
                # Nothing to insert in pass 1 — no undeclared vars.
                # Check if pass 2 would find anything. If the file already
                # has no undeclared and no unused, we're done.
                continue
            break

        # Apply fixes in reverse position order
        fixes.sort(key=lambda x: -x[0])

        for pos, code, reason in fixes:
            content = content[:pos] + code + content[pos:]
            print(f"  {filepath.name}: {reason}")
            modified = True

        # Write back after each pass so pass 2 sees updated content
        filepath.write_text(content, encoding="utf-8")

    return modified


# ── Manual / special fixes ──


def apply_manual_fixes():
    """Apply manual fixes for patterns too complex for automated analysis."""

    # ── test_builtins_part9.js: cross-fragment function reference ──
    # degToRad declared in fragment 92 (nested function), referenced in fragment 93
    # Fix: move function declarations to before all fragments
    part9 = JS_SRC / "test_builtins_part9.js"
    if part9.exists():
        content = part9.read_text(encoding="utf-8")

        # Check if already fixed
        if "// Hoisted cross-fragment function declarations" not in content:
            # Find fragment 92's function declarations
            old_pattern = (
                "function test_builtins_part9() {\n"
                "// ---- fragment 92 ----\n"
                "try {{\n"
                "        function degToRad(degrees) {\n"
                "          return degrees * (Math.PI / 180);\n"
                "        }\n"
                "\n"
                "        function radToDeg(rad) {\n"
                "          return rad / (Math.PI / 180);\n"
                "        }\n"
            )
            new_pattern = (
                "function test_builtins_part9() {\n"
                "// Hoisted cross-fragment function declarations\n"
                "function degToRad(degrees) {\n"
                "  return degrees * (Math.PI / 180);\n"
                "}\n"
                "\n"
                "function radToDeg(rad) {\n"
                "  return rad / (Math.PI / 180);\n"
                "}\n"
                "\n"
                "// ---- fragment 92 ----\n"
                "try {{\n"
            )

            if old_pattern in content:
                content = content.replace(old_pattern, new_pattern)
                part9.write_text(content, encoding="utf-8")
                print("  test_builtins_part9.js: hoisted degToRad/radToDeg to outer scope")

    # ── test_builtins_part15.js: 'array' undeclared in fragments 159, 160 ──
    # The automated fix should handle this, but verify

    # ── test_expressions_part1.js: fragment 5 has doSomething() + checkbox ──
    # The automated fix handles doSomething (function) and checkbox (object)

    # ── test_expressions_part1.js: fragment 1 has `void expression` — needs
    # `var expression = 0;` — automated fix handles this

    # ── test_builtins_part11.js: 'try' outside function scope ──
    # This is a Rust-side codegen bug with `str.split(regex)` generating try
    # in return type expression. NOT fixable in JS source.
    # TODO: fix in codegen/expr.rs

    # ── test_statements_part3.js: globalThis.contains(...) ──
    # Automated fix handles this via `var globalThis = {};`


def main():
    """Run manual + automated fixes on all MDN part files."""
    # Phase 1: Manual fixes first (so automated fix sees hoisted declarations)
    print("=" * 60)
    print("Phase 1: Manual fixes (cross-fragment refs, special patterns)")
    print("=" * 60)
    apply_manual_fixes()

    # Phase 2: Automated fixes for undeclared/unused variables
    print(f"\n{'=' * 60}")
    print("Phase 2: Auto-fix undeclared + unused variables")
    print("=" * 60)

    fixed_count = 0
    total_files = 0

    for js_file in sorted(JS_SRC.glob("test_*_part*.js")):
        total_files += 1
        if apply_fixes_to_file(js_file):
            fixed_count += 1

    print(f"\nAutomated fixes: {fixed_count}/{total_files} files modified")

    print(f"\n{'=' * 60}")
    print("Done. Re-run `cargo build` in mdn-test-project to verify.")
    print("=" * 60)


if __name__ == "__main__":
    main()
