# TagSpeak Agents

This file documents the **agents** (packet modules) available in TagSpeak.\
Each agent is a symbolic `[packet]` that can be chained with `>` or grouped with `{ ... }`.\
Everything in TagSpeak is a packet.\
\
&#x20;   [note@"This section is about 4-5 pages long."]

---

## 🌊 Flow of TagSpeak

TagSpeak is a **dataflow language**.\
Unlike imperative languages where control is the focus, TagSpeak emphasizes the flow of values from source → transformation → action.

**Execution Flow**

```
[packet source code]  
   ↓ (lex/parse packets)  
AST (Node tree: Chain, Packet, If, Block, etc.)  
   ↓ (dispatch nodes)  
Router (maps AST nodes to packet modules)  
   ↓ (execute)  
Runtime (manages variables, tags, last value, state)  
   ↓  
Packet Handlers (math, store, print, loop, conditionals, etc.)
```

**Grammar Flow**

```
[data] > [modify_data] > [final_action]
```

**Example**

```tgsk
[math@5+5] > [print]
```

*Read aloud: “Take this data (5+5), then tell me the result.”*

---

## 📦 Core Packets

### `[math@expr]`

- **Role:** Evaluate arithmetic expressions.
- **Input:** Expression string (`5*2`, `10+3`, etc.).
- **Output:** `Value::Num(f64)`.
- **Example:**
  ```tgsk
  [math@5*2] > [print]
  ```

---

### `[int@num]`

- **Role:** Return an integer constant.
- **Input:** Integer literal or concatenation expression.
- **Output:** `Value::Num(f64)` (integer form).
- **Examples:**
  ```tgsk
  [int@5] > [store@x]
  [int@5+10] > [print]
  ```

---

### `[bool@true|false]`

- **Role:** Return a boolean constant.
- **Input:** `true` or `false`, or boolean expression.
- **Output:** `Value::Bool`.
- **Examples:**
  ```tgsk
  [bool@true] > [store@flag]
  [bool@false] > [store@!flag]
  ```

---

### `[store[:mode]@var]`

- **Role:** Save the last pipeline value into a variable.
- **Input:** Variable name.
- **Output:** Stored value (or `Unit`).

**Modes (nuance labels):**

| Form                      | Meaning       | Behavior                                | Example                                                                               |
| ------------------------- | ------------- | --------------------------------------- | ------------------------------------------------------------------------------------- |
| `[store:rigid@x]`         | Rigid (const) | Immutable after first set.              | `[math@42] > [store:rigid@answer]`                                                    |
| `[store:fluid@x]`         | Fluid (let)   | Mutable; can be reassigned.             | `[math@0] > [store:fluid@counter]`                                                    |
| `[store:context(cond)@x]` | Contextual    | Value chosen dynamically when recalled. | `[store:context(user_angry==true > user_frustrated==true)@tone] > [msg@"apologetic"]` |

**Context syntax:**

- `a==b > c==d` → OR (either true).
- `a==b, c==d` → AND (both true).
- `(default==true)` → fallback if no other matches.
- Under the hood explanation: behaves like sugar for [if] statements directly within [store]

---

### `[print@value]`

- **Role:** Print a string or the current pipeline value.
- **Example:**
  ```tgsk
  [print@"hi"]  
  [math@1+1] > [print]
  ```

---

### `[msg@text]`

- **Role:** Return a string value to the pipeline (instead of printing). Useful for AI/LLM tools.
- **Input:** String literal or concatenation expression.
- **Output:** `Value::Str`.
- **Examples:**
  ```tgsk
  [msg@"hello"] > [store@x]
  [msg@"hello"+"hello"] > [print] //concatnation is built in.
  ```

---

### `[note@"text"]`

- **Role:** Debug/developer annotation.
- **Output:** Only visible in debug mode.

---

### `[funct:tag]{ ... }`

- **Role:** Define a reusable block of packets.
- **Example:**
  ```tgsk
  [funct:step]{ [print@"inside step"] }
  ```

---

### `[loop]`

- **Role:** Repeat a block N times.
- **Forms:**
  - Inline: `[loop@3]{ ... }`
  - Named: `[funct:step]{ ... } … [loop3@step]`

---

### `[load@file]` / `[save@file]`

- **Role:** File I/O.
- `[load]` → read file → value.
- `[save]` → write last value → file.
- **Example:**
  ```tgsk
  [load@data.json] > [print]  
  [math@99] > [save@result.txt]
  ```

---

### `[log@file]`

- **Role:** Log the last pipeline value into a JSON file.
- **Input:** File path.
- **Output:** `Unit`.
- **Notes:** Behaves like `[save]` but enforces JSON structure. If file does not exist, it creates one.
- **Example:**
  ```tgsk
  [math@42] > [log@result.json]
  ```

---

### `[mod@Var]{ ... }`

- **Role:** Open a `Document` for JSON edits.
- **Inner ops:**
  - `[comp(path)@val]` → replace
  - `[comp!(path)@val]` → replace or create parents
  - `[merge(path)@{...}]` → deep merge
  - `[del(path)]` → delete
  - `[ins(path)@val]` → insert new

---

## 🧭 Conditionals

### `[if] / [or] / [then] / [else]`

- **Role:** Branch execution.
- **Grammar:**
  ```tgsk
  [if@(cond)] > [then]{ ... }
    > [or@(cond)] > [then]{ ... }
    > [else]{ ... }
  ```
- **Example:**
  ```tgsk
  [if@(x==1)] > [then]{ [print@"one"] }
    > [or@(x==2)] > [then]{ [print@"two"] }
    > [else]{ [print@"other"] }
  ```

---

## ⚖️ Comparison Operators

| Alias  | Canon  | Meaning          |
| ------ | ------ | ---------------- |
| `[=]`  | `[eq]` | equal            |
| `[!=]` | `[ne]` | not equal        |
| `[>]`  | `[gt]` | greater than     |
| `[>=]` | `[ge]` | greater or equal |
| `[<]`  | `[lt]` | less than        |
| `[<=]` | `[le]` | less or equal    |

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
2. User-dependent flow: sugar (`==`) and canonical packets (`[eq]`) coexist.
3. Packets must be human-readable and machine-parsable.

---

# LLM Format

```tgsk
[doc:TagSpeak_Agents]{
  [note@"Codeblock = 50 lines"]
  [section:flow]{
    [llm_note@"If you are an LLM parsing this doc, skip directly to this section"]
    [note@"Execution pipeline: source > AST > router > runtime > handlers"]
    [example]{ [math@5+5] > [print] }
  }

  [section:core]{
    [agent:math]{
      [role@"Arithmetic evaluation"]
      [input@"expr string (5*2, 10+3)"]
      [output@"Value::Num(f64)"]
      [example]{ [math@5*2] > [print] }
    }

    [agent:int]{
      [role@"Integer literal"]
      [output@"Value::Num(f64)"]
      [example]{ [int@5+10] > [print] }
    }

    [agent:bool]{
      [role@"Boolean literal"]
      [output@"Value::Bool"]
      [example]{ [bool@true+false] > [print] }
    }

    [agent:store]{
      [role@"Variable storage"]
      [mode:rigid@"Immutable const"]
      [mode:fluid@"Mutable let"]
      [mode:context@"Conditional overlay"]
      [example:rigid]{ [math@42] > [store:rigid@answer] }
      [example:fluid]{ [math@0] > [store:fluid@counter] }
      [example:context]{ [store:context(user_angry==true > user_frustrated==true)@tone] > [msg@"apologetic"] }
      [example:and]{ [store:context(user_logged_in==true, user_admin==true)@permissions] > [msg@"elevated"] }
    }

    [agent:print]{
      [role@"Print value or string"]
      [example]{ [print@"hi"] }
    }

    [agent:msg]{
      [role@"Return string value to pipeline"]
      [example]{ [msg@"hello"+"world"] > [print] }
    }

    [agent:note]{ [role@"Debug annotation"] }

    [agent:funct]{
      [role@"Reusable block"]
      [example]{ [funct:step]{ [print@"inside step"] } }
    }

    [agent:loop]{
      [role@"Repeat block"]
      [form:inline@"[loop@3]{...}"]
      [form:named@"[funct:step]{...} … [loop3@step]"]
    }

    [agent:load]{ [role@"Read file"] }
    [agent:save]{ [role@"Write file"] }
    [agent:log]{
      [role@"Write pipeline value to JSON file"]
      [example]{ [math@42] > [log@result.json] }
    }
    [agent:mod]{ [role@"Modify JSON document"] }
  }

  [section:conditionals]{
    [agent:if]{
      [role@"Conditional branching"]
      [grammar@"[if@(cond)] > [then]{...} > [or@(cond)] > [then]{...} > [else]{...}"]
      [example]{ [if@(x==1)] > [then]{ [print@"one"] } > [or@(x==2)] > [then]{ [print@"two"] } > [else]{ [print@"other"] } }
    }
  }

  [section:comparisons]{
    [alias@"[=] is [eq]"]
    [alias@"[!=] is [ne]"]
    [alias@"[>] is [gt]"]
    [alias@"[>=] is [ge]"]
    [alias@"[<] is [lt]"]
    [alias@"[<=] is [le]"]
    [example]{ [if@(x[>]10)] > [then]{ [print@"big"] } }
  }

  [section:truthiness]{
    [rule@"Bool(true/false) → itself"]
    [rule@"Num nonzero → true"]
    [rule@"Str nonempty → true"]
    [rule@"Unit → false"]
  }

  [section:design_rules]{
    [rule@"Everything is a packet"]
    [rule@"Sugar + canonical coexist"]
    [rule@"Readable + parsable"]
  }
}
```

