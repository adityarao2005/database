# Mermaid Railroad EBNF Diagram Guide (`railroad-ebnf-beta`)

Mermaid's `railroad-ebnf-beta` renders formal language grammars, command line tools, and file formats into visual syntax diagrams (railroad tracks).

---

## 1. Syntax Rules & Operators

Mermaid's railroad EBNF parser is based on **ISO/IEC 14977 Standard EBNF**, **NOT** Regex or PEG!

| Construct | EBNF Syntax in Mermaid | INCORRECT / Regex (Will Fail Parser) | Meaning |
| :--- | :--- | :--- | :--- |
| **Optional** | `[ item ]` | `item?` or `(item)?` | Item occurs 0 or 1 time |
| **Repetition (0+)** | `{ item }` | `item*` or `(item)*` | Item repeats 0 or more times |
| **Choice / Alternative** | `a \| b` | `a / b` | Either `a` or `b` |
| **Grouping** | `( a b )` | - | Groups elements together |
| **Terminals (Literals)**| `"literal"` or `'literal'` | `literal` (without quotes is non-terminal) | Literal string tokens |
| **Rule End** | `rule = ... ;` | missing `;` | Every rule must terminate with `;` |
| **Title** | `title "My Grammar"` | `title My Grammar` | Enclose titles with spaces in quotes |
| **Comments** | `%% comment text` | `// comment` or `# comment` | Lines starting with `%%` |

> [!CAUTION]
> **Never use `?`, `*`, or `+` as postfix operators in `railroad-ebnf-beta`!**
> EBNF treats `?` and `*` differently than regular expressions. Placing `?` or `*` after symbols or closing parentheses causes Chevron/Nearley parser exceptions like `Expecting token of type ')' but found ';'` or `Expecting token of type ';' but found ')'`.

---

## 2. Common Patterns & Cheatsheet

### Optional Elements
```ebnf
%% Optional distinct keyword
select_stmt = "select" [ "distinct" ] columns ;

%% Nested optional (e.g. alias)
column_item = column_name [ [ "as" ] alias_name ] ;
```

### Comma-Separated Lists (Zero or More, One or More)
```ebnf
%% One or more comma-separated items: item { "," item }
items_list = item { "," item } ;

%% Zero or more comma-separated items: [ item { "," item } ]
optional_list = [ item { "," item } ] ;
```

### Whitespace Separators
If whitespace is explicitly modeled in your grammar:
```ebnf
select = "select" whitespace columns [ whitespace "from" whitespace table ] ;
items = item { [ whitespace ] "," [ whitespace ] item } ;
```

### Delimiter Spacing
Always leave spaces before the terminal semicolon ` ;` to ensure token boundaries are cleanly parsed:
```ebnf
rule_name = term1 ( term2 | term3 ) [ optional_term ] ;
```

---

## 3. Complete Working Example

```mermaid
railroad-ebnf-beta
    title "JSON Syntax Grammar"

    value = object | array | string | number | "true" | "false" | "null" ;

    object = "{" [ pair { "," pair } ] "}" ;
    pair = string ":" value ;

    array = "[" [ value { "," value } ] "]" ;
```

