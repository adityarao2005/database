---
name: mermaid-diagrams
description: >-
  Use this skill whenever creating, modifying, linting, or verifying Mermaid diagrams
  (including flowcharts, sequence diagrams, class diagrams, state diagrams, ER diagrams,
  and railroad-ebnf-beta syntax diagrams) in Markdown documentation or codebases.
---

# Mermaid Diagram Creation & Verification

This skill provides runbooks, syntax rules, and an automated verification script to create robust, error-free Mermaid diagrams and avoid preview parse failures.

---

## 1. Quick Verification Command

Before presenting or committing any modified or newly created Mermaid diagrams, run the verification script:

```bash
python3 .agents/skills/mermaid-diagrams/scripts/verify_mermaid.py [path/to/file.md]
```

To validate all Markdown and diagram files across the workspace:
```bash
python3 .agents/skills/mermaid-diagrams/scripts/verify_mermaid.py
```

The script verifies:
- Balanced parentheses `()`, brackets `[]`, and braces `{}`.
- Unquoted special characters inside flowchart node labels.
- Unclosed blocks (`alt`, `opt`, `loop`, `par`) in sequence diagrams.
- Proper ISO EBNF syntax in `railroad-ebnf-beta` diagrams (flagging illegal regex postfixes `?`, `*`, `+`).

---

## 2. Critical Syntax Traps by Diagram Type

### A. Railroad Syntax Diagrams (`railroad-ebnf-beta`)
See detailed guide: [railroad_ebnf.md](./references/railroad_ebnf.md)

- **Optional items**: Use square brackets `[ ... ]`. **Never** use `?` or `(...)?`.
- **Repetitions (0 or more)**: Use curly braces `{ ... }`. **Never** use `*` or `(...)*`.
- **Terminals**: Enclose literal tokens in quotes: `"select"`, `";"`, `","`.
- **Title**: Enclose title strings with spaces in double quotes: `title "My Grammar"`.
- **Rule termination**: Every rule statement must conclude with a semicolon separated by whitespace: `rule_name = ... ;`.

### B. Flowcharts (`flowchart TD` / `flowchart LR`)
See detailed guide: [common_diagrams.md](./references/common_diagrams.md)

- **Node label quoting**: Any label containing parentheses `()`, brackets `[]`, braces `{}`, or punctuation **must** be wrapped in double quotes:
  - ❌ Incorrect: `node[My Node (Detail)]`
  - ✅ Correct: `node["My Node (Detail)"]`
- **Link labels**: Format text on links as `-->|label|` or `-- label -->`.

### C. Sequence Diagrams (`sequenceDiagram`)
See detailed guide: [common_diagrams.md](./references/common_diagrams.md)

- **Block termination**: Every `alt`, `opt`, `loop`, `par`, and `critical` block must end with an explicit `end`.
- **Arrow types**:
  - `->>`: Synchronous request (solid line, filled head).
  - `-->>`: Response / Return message (dashed line, open head).

### D. Class Diagrams (`classDiagram`)
See detailed guide: [common_diagrams.md](./references/common_diagrams.md)

- **Relationship arrows**: `<|--` (inheritance), `*--` (composition), `o--` (aggregation), `-->` (association).
- **Visibility**: `+` (public), `-` (private), `#` (protected), `~` (package).

---

## 3. Workflow for Writing Diagrams

1. **Choose the appropriate diagram type**:
   - Syntax & grammars: `railroad-ebnf-beta`
   - Workflows & architecture: `flowchart TD` or `flowchart LR`
   - Component interactions & protocols: `sequenceDiagram`
   - Data models & schemas: `classDiagram` or `erDiagram`
   - State lifecycles: `stateDiagram-v2`
2. **Draft the diagram** following the cheatsheets in `./references/`.
3. **Execute verification**:
   ```bash
   python3 .agents/skills/mermaid-diagrams/scripts/verify_mermaid.py path/to/document.md
   ```
4. Address any flagged lint issues before committing or responding to the user.

