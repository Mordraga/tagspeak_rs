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

### [load]
- **Role:** Read data from a file.
- **Input:** File path.
- **Output:** File contents as a value.
- **Example:**
  ```tgsk
  [load@data.json] > [print]
  ```

### [save]
- **Role:** Write the last value to a file.
- **Input:** File path or `as@file` when saving a variable.
- **Output:** `Unit`.
- **Example:**
  ```tgsk
  [math@1+1] > [save@result.txt]
  ```

### [mod]
- **Role:** Open a stored value for modification.
- **Usage:** `[mod@Var]{ ... }` where inner packets mutate `Var`.
- **Inner Packets:** `[comp]`, `[comp!]`, `[merge]`, `[del]`, `[ins]`.

| operation | behavior |
|-----------|----------|
| `comp`  | Replace value at the path; the parent must already exist. |
| `comp!` | Replace value, creating any missing parents along the path (object only). |
| `merge` | Deep-merge an object into the existing object at the path (object only). |
| `del`   | Delete the value at the path; error if missing. |
| `ins`   | Insert a new value; error if the path already exists. |

### [merge]
- **Role:** Combine two JSON/YAML/TOML structures.
- **Input:** Source value to merge into the current context.
- **Behavior:** Keys from the source overwrite or extend the target.
- **Example:**
  ```tgsk
  [load@first.json]  > [save@A]
  [load@second.json] > [save@B]

  [mod@A]{ [merge(.)@B] }
    > [save(as@merged.json)@A]
  ```

### [ins]
- **Role:** Insert or replace data inside the current value opened by `[mod]`.
- **Input:** Structure to insert (`{ key: value }`, array, etc.).
- **Behavior:** Adds new keys or indices; overwrites existing ones.
- **Example:**
  ```tgsk
  [load@profile.json] > [save@P]

  [mod@P]{ [ins(.)@{ city:"Wonderland", hobby:"Adventuring" }] }
    > [save(as@profile.json)@P]
  ```

### [del]
- **Role:** Remove a key or index from the current value opened by `[mod]`.
- **Input:** Path to delete (`key`, `path.to.key`, `3`, etc.).
- **Behavior:** Deletes the specified field or element if it exists.
- **Example:**
  ```tgsk
  [load@profile.json] > [save@P]

  [mod@P]{ [del@obsolete] }
    > [save(as@clean.json)@P]
  ```

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

