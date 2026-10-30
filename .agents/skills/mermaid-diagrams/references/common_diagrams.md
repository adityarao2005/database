# Common Mermaid Diagram Types & Syntax Reference

Quick reference for writing clean, valid Mermaid diagrams across all supported diagram types.

---

## 1. Flowcharts (`flowchart TD` / `flowchart LR`)

### Direction
- `TD` or `TB`: Top to Bottom
- `LR`: Left to Right
- `BT`: Bottom to Top
- `RL`: Right to Left

### Node Shapes & Special Character Quoting
> [!IMPORTANT]
> If a node label contains parentheses `()`, brackets `[]`, braces `{}`, or quotes, **always wrap the label in double quotes** inside the shape delimiters!

```mermaid
flowchart LR
    rect[Standard Rectangle]
    round(Rounded Rectangle)
    decision{Decision Rhombus}
    db[("Database Cylinder")]
    quoted["Node with (Parentheses) & Extra Info"]
```

### Links & Arrow Syntax
- Normal: `-->`
- Open link: `---`
- Text on link: `-->|label|` or `-- label -->`
- Dotted: `-.->` or `-. text .->`
- Thick: `==>` or `== text ==>`

---

## 2. Sequence Diagrams (`sequenceDiagram`)

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Client
    participant Server

    User->>Client: Click Button
    activate Client
    Client->>Server: POST /api/data
    activate Server
    Server-->>Client: 200 OK (JSON)
    deactivate Server
    Client-->>User: Render View
    deactivate Client

    alt Valid Input
        Client->>Server: Commit
    else Invalid Input
        Client->>User: Show Error
    end
```

### Gotchas:
- All blocks (`alt`, `opt`, `loop`, `par`, `critical`, `rect`) **must** be terminated with a matching `end`.
- Use `->>` for synchronous messages (solid arrow with filled head), `-->>` for reply messages (dashed arrow with open head).

---

## 3. Class Diagrams (`classDiagram`)

```mermaid
classDiagram
    class Animal {
        +String name
        +int age
        +makeSound() void
    }

    class Dog {
        +fetch() void
    }

    Animal <|-- Dog : Inheritance
    House *-- Room : Composition
    Car o-- Wheel : Aggregation
```

### Visibility Modifiers:
- `+`: Public
- `-`: Private
- `#`: Protected
- `~`: Package / Internal

---

## 4. Entity-Relationship Diagrams (`erDiagram`)

```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ ORDER_ITEM : contains
    PRODUCT ||--o{ ORDER_ITEM : ordered_in

    CUSTOMER {
        int id PK
        string email
        string name
    }
    ORDER {
        int id PK
        int customer_id FK
        datetime created_at
    }
```

### Cardinality Notations:
- `||`: Exactly one
- `|{` / `}|`: One or more
- `|o` / `o|`: Zero or one
- `o{` / `}o`: Zero or more
