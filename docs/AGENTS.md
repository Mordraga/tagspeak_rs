# TagSpeak Agents

This file documents the **agents** (packet modules) available in TagSpeak.  
Each agent is a symbolic `[packet]` that can be chained with `>` or grouped with `{ ... }`.  
Everything in TagSpeak is a packet.

---
## Flow of TagSpeak

TagSpeak is a dataflow language.
Unlike imperative languages where control is the focus, TagSpeak emphasizes the flow of values from source → transformation → action.

### Execution Flow

[packet source code]  
   ↓ (lex/parse packets)  
AST (Node tree: Chain, Packet, If, Block, etc.)  
   ↓ (dispatch nodes)  
Router (maps AST nodes to packet modules)  
   ↓ (execute)  
Runtime (manages variables, tags, last value, state)  
   ↓  
Packet Handlers (math, store, print, loop, conditionals, etc.)


### Grammar Flow

[data] > [modify_data] > [final_action]


Example:

[math@5+5] > [print]


Read aloud: “Take this data (5+5), then tell me the result.”
---
## Core Packets

### [math]
- **Role:** Evaluate arithmetic expressions.
- **Input:** Expression string (`5*2`, `10+3`, etc.).
- **Output:** `Value::Num(f64)`.
- **Example:**  
  ```tgsk
  [math@5*2] > [print]
  ```

### [store]
- **Role:** Assign the last pipeline value to a variable.
- **Input:** Variable name.
- **Output:** Stored value (or `Unit`).
- **Example:**  
  ```tgsk
  [math@42] > [store@answer]
  ```

### [print]
- **Role:** Print a string or the current pipeline value.
- **Input:** Optional string or variable reference.
- **Output:** Prints to stdout, returns `Unit`.
- **Example:**  
  ```tgsk
  [print@"hello"]
  [math@1+1] > [print]
  ```

### [note]
- **Role:** Developer/debug annotation.  
- **Output:** Prints only in debug/dev mode.  

### [funct]
- **Role:** Define a reusable block of packets.
- **Example:**  
  ```tgsk
  [funct:step]{ [print@"inside step"] }
  ```

### [loop]
- **Role:** Repeat a block N times.  
- **Forms:**
  - `[loop@3]{ ... }` (inline loop)  
  - `[funct:step]{ ... } … [loop3@step]` (named loop)

---

## Conditionals

### [if] / [then] / [or] / [else]
- **Role:** Conditional chains.  
- **Grammar:**  
  ```tgsk
  [if@(cond)] > [then]{ ... }
    > [or@(cond)] > [then]{ ... }
    > [else]{ ... }
  ```
- **Output:** Last value of executed branch or `Unit`.

**Examples:**
```tgsk
[if@(x==1)] > [then]{ [print@"one"] }
  > [or@(x==2)] > [then]{ [print@"two"] }
  > [else]{ [print@"other"] }
```

---

## Comparison Operators

Packets and symbolic aliases are interchangeable:

| Alias   | Packet | Meaning                  |
|---------|--------|--------------------------|
| `[=]`   | `[eq]` | equal                    |
| `[!=]`  | `[ne]` | not equal                |
| `[>]`   | `[gt]` | greater than             |
| `[>=]`  | `[ge]` | greater or equal         |
| `[<]`   | `[lt]` | less than                |
| `[<=]`  | `[le]` | less or equal            |
---
**Behavior**
Acts as packets for the sake of usability, allowing users to define them as variables.
Such as:
```tgsk
{[ne]>[lt]}>[store@customCompare]
```

**Example:**
```tgsk
[if@(x[>]10)] > [then]{ [print@"big"] }
[if@(x[eq]10)] > [then]{ [print@"equal"] }
```

---

## Truthiness

- `Bool(true/false)` → itself  
- `Num` → true if not zero and not NaN  
- `Str` → true if non-empty  
- `Unit` → false  

---

## Design Rules

1. Everything is a packet.  
2. User-dependent flow: sugar (e.g. `==`) and canonical packets (`[eq]`) coexist.  
3. Packets must be human-readable and machine-parsable.

