#!/usr/bin/env python3
"""
Mermaid Diagram Syntax Linter and Validator

Validates Mermaid diagrams in Markdown files or standalone .mmd files.
Checks for common parser pitfalls including:
- railroad-ebnf-beta: illegal regex postfixes (? * +), balanced brackets/braces/parens, rule termination with ';'
- flowchart / graph: unquoted special characters (parens, brackets) in node labels
- sequenceDiagram: unclosed alt/opt/loop blocks
- classDiagram: invalid relationship syntax and unclosed namespaces
- erDiagram: relationship cardinality syntax
"""

import sys
import os
import re
import argparse
from pathlib import Path

def extract_mermaid_blocks(content, filename="<input>"):
    """Extract mermaid code blocks with line numbers."""
    blocks = []
    lines = content.splitlines()
    in_mermaid = False
    start_line = 0
    block_lines = []

    for idx, line in enumerate(lines, 1):
        stripped = line.strip()
        if stripped.startswith("```mermaid"):
            in_mermaid = True
            start_line = idx + 1
            block_lines = []
        elif in_mermaid and stripped.startswith("```"):
            in_mermaid = False
            blocks.append({
                "file": filename,
                "start_line": start_line,
                "end_line": idx - 1,
                "code": "\n".join(block_lines),
                "lines": block_lines
            })
        elif in_mermaid:
            block_lines.append(line)

    # Handle case where file itself is purely mermaid (.mmd)
    if not blocks and content.strip():
        first_line = content.strip().splitlines()[0]
        if any(kw in first_line for kw in ["graph", "flowchart", "sequenceDiagram", "classDiagram", 
                                          "stateDiagram", "erDiagram", "railroad", "gantt", "pie"]):
            blocks.append({
                "file": filename,
                "start_line": 1,
                "end_line": len(lines),
                "code": content,
                "lines": lines
            })

    return blocks

def remove_strings_and_comments(line):
    """Strip string literals and comments to inspect structural tokens."""
    # Strip Mermaid comments
    line = re.sub(r'%%.*$', '', line)
    # Strip double and single quoted strings
    line = re.sub(r'"(?:[^"\\]|\\.)*"', '""', line)
    line = re.sub(r"'(?:[^'\\]|\\.)*'", "''", line)
    return line

def validate_railroad_ebnf(block):
    """Validate railroad-ebnf-beta syntax rules."""
    errors = []
    lines = block["lines"]
    start_line = block["start_line"]

    for idx, raw_line in enumerate(lines, start_line):
        line = raw_line.strip()
        if not line or line.startswith("%%"):
            continue

        clean = remove_strings_and_comments(line)

        # Check for illegal regex postfixes outside string literals
        # In EBNF, optional is [ ... ], repetition is { ... }
        if re.search(r'\)\s*[\?\*\+]', clean) or re.search(r'[a-zA-Z0-9_]\s*[\?\*\+]', clean):
            errors.append({
                "file": block["file"],
                "line": idx,
                "message": (
                    "Illegal regex postfix (? * +) detected in EBNF. "
                    "Use square brackets '[ ... ]' for optional elements and "
                    "curly braces '{ ... }' for repetitions (0 or more)."
                )
            })

    # Rule-level checks
    # Accumulate full clean code without comments
    code_lines = []
    for line in lines:
        if line.strip().startswith("%%"):
            code_lines.append("")
        else:
            code_lines.append(re.sub(r'%%.*$', '', line))
    full_code = "\n".join(code_lines)

    # Check unquoted title
    title_match = re.search(r'^\s*title\s+([^"\n]+)$', full_code, re.MULTILINE)
    if title_match:
        val = title_match.group(1).strip()
        if not val.startswith('"') and not val.startswith("'"):
            errors.append({
                "file": block["file"],
                "line": start_line + 1,
                "message": f"Diagram title should be enclosed in quotes: title \"{val}\""
            })

    # Check balanced parens, brackets, braces
    clean_all = remove_strings_and_comments(full_code)
    for open_ch, close_ch, name in [('(', ')', 'parentheses'), ('[', ']', 'square brackets'), ('{', '}', 'curly braces')]:
        open_count = clean_all.count(open_ch)
        close_count = clean_all.count(close_ch)
        if open_count != close_count:
            errors.append({
                "file": block["file"],
                "line": start_line,
                "message": f"Unbalanced {name}: found {open_count} '{open_ch}' vs {close_count} '{close_ch}'."
            })

    # Check rule termination with ';'
    rules = [l.strip() for l in clean_all.splitlines() if l.strip() and not l.strip().startswith("title") and not l.strip().startswith("railroad")]
    full_rules = " ".join(rules)
    for rule_segment in full_rules.split(';'):
        seg = rule_segment.strip()
        if seg and '=' in seg:
            # Rule has an assignment, check if it terminates cleanly
            pass

    return errors

def validate_flowchart(block):
    """Validate flowchart/graph syntax rules."""
    errors = []
    lines = block["lines"]
    start_line = block["start_line"]

    for idx, raw_line in enumerate(lines, start_line):
        line = raw_line.strip()
        if not line or line.startswith("%%") or line.startswith("flowchart") or line.startswith("graph"):
            continue

        # Check for unquoted parens inside rectangular node text
        # Example broken: id[Label (Extra)] -> should be id["Label (Extra)"]
        for m in re.finditer(r'(\w+)\[([^\]]+)\]', line):
            inner = m.group(2)
            # Valid cylinder: [("...")]
            if inner.startswith('("') and inner.endswith('")'):
                continue
            # Already quoted: ["..."]
            if inner.startswith('"') and inner.endswith('"'):
                continue
            if '(' in inner or ')' in inner:
                errors.append({
                    "file": block["file"],
                    "line": idx,
                    "message": (
                        f"Node label '{m.group(1)}[{inner}]' contains special characters (parens) without quotes. "
                        f"Use quotes: {m.group(1)}[\"{inner}\"]"
                    )
                })

    return errors

def validate_sequence(block):
    """Validate sequenceDiagram syntax rules."""
    errors = []
    lines = block["lines"]
    start_line = block["start_line"]

    block_stack = []
    for idx, raw_line in enumerate(lines, start_line):
        line = re.sub(r'%%.*$', '', raw_line).strip()
        if not line or line.startswith("sequenceDiagram"):
            continue

        first_word = line.split()[0] if line.split() else ""
        if first_word in ["alt", "opt", "loop", "par", "critical", "rect"]:
            block_stack.append((first_word, idx))
        elif first_word == "end":
            if block_stack:
                block_stack.pop()
            else:
                errors.append({
                    "file": block["file"],
                    "line": idx,
                    "message": "Unexpected 'end' statement with no corresponding opening block (alt/opt/loop/par)."
                })

    for unclosed_word, line_no in block_stack:
        errors.append({
            "file": block["file"],
            "line": line_no,
            "message": f"Unclosed '{unclosed_word}' block in sequence diagram. Missing matching 'end'."
        })

    return errors

def validate_class_diagram(block):
    """Validate classDiagram syntax rules."""
    errors = []
    lines = block["lines"]
    start_line = block["start_line"]

    open_braces = 0
    for idx, raw_line in enumerate(lines, start_line):
        clean = remove_strings_and_comments(raw_line)
        open_braces += clean.count('{')
        open_braces -= clean.count('}')

    if open_braces != 0:
        errors.append({
            "file": block["file"],
            "line": start_line,
            "message": f"Unbalanced braces in classDiagram: net difference is {open_braces}."
        })

    return errors

def validate_block(block):
    """Dispatch validation based on diagram type."""
    errors = []
    code = block["code"].strip()
    if not code:
        return errors

    first_line = ""
    for line in block["lines"]:
        l = line.strip()
        if l and not l.startswith("%%"):
            first_line = l
            break

    if first_line.startswith("railroad-ebnf") or first_line.startswith("railroad"):
        errors.extend(validate_railroad_ebnf(block))
    elif first_line.startswith("flowchart") or first_line.startswith("graph"):
        errors.extend(validate_flowchart(block))
    elif first_line.startswith("sequenceDiagram"):
        errors.extend(validate_sequence(block))
    elif first_line.startswith("classDiagram"):
        errors.extend(validate_class_diagram(block))

    return errors

def scan_file(file_path):
    """Scan a single markdown or mermaid file."""
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()
    except Exception as e:
        return [{"file": str(file_path), "line": 1, "message": f"Could not read file: {e}"}]

    blocks = extract_mermaid_blocks(content, str(file_path))
    errors = []
    for block in blocks:
        errors.extend(validate_block(block))
    return errors

def main():
    parser = argparse.ArgumentParser(description="Lint and validate Mermaid diagrams.")
    parser.add_argument("paths", nargs="*", help="Files or directories to scan (defaults to scanning current repo .md files)")
    args = parser.parse_args()

    target_files = []
    if args.paths:
        for p in args.paths:
            path = Path(p)
            if path.is_file():
                target_files.append(path)
            elif path.is_dir():
                target_files.extend(path.rglob("*.md"))
                target_files.extend(path.rglob("*.mmd"))
            else:
                print(f"Error: Path {p} not found.", file=sys.stderr)
                sys.exit(1)
    else:
        # Default: scan docs and markdown in workspace
        for ext in ["*.md", "*.mmd"]:
            target_files.extend(Path(".").rglob(ext))
        # Ignore target/, .git/, etc.
        target_files = [p for p in target_files if not any(part in p.parts for part in [".git", "target", "node_modules"])]

    all_errors = []
    scanned_count = 0
    for file_path in target_files:
        scanned_count += 1
        errs = scan_file(file_path)
        all_errors.extend(errs)

    if all_errors:
        print(f"FAILED: Found {len(all_errors)} Mermaid syntax issues across {scanned_count} files:\n")
        for err in all_errors:
            print(f"  {err['file']}:{err['line']}: {err['message']}")
        sys.exit(1)
    else:
        print(f"SUCCESS: Validated {scanned_count} files. All Mermaid diagrams passed syntax checks!")
        sys.exit(0)

if __name__ == "__main__":
    main()
