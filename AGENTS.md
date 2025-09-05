# AGENTS.md

## Purpose

This repository defines **TagSpeak (DSL)**: a dataflow-oriented packet language designed for AI ↔ human accessibility.
Codex should treat `.tgsk` files as **DSL**, not general-purpose code.

---

## Instructions for Codex

1. ### **Parsing**

   * Everything is a `[packet]`.
   * Flow is data-driven, not imperative.
   * Inline sugar (like `==`) coexists with canonical packets (e.g. `[eq]`).

2. ### **File Types**

   * `.tgsk` → TagSpeak source files.
   * `.json` → runtime/memory store.
   * `.yaml` / `.yml` → runtime/memory store.
   * `.toml` → runtime/memory store.
   * `.rs` → interpreter/compiler backend (Rust).

3. ### **Behavior**

   * Do not “correct” DSL syntax to Python/Rust.
   * When asked for code examples, prefer `[math@5+5]` style over verbose boilerplate.
   * Normalize to LF endings.
   * For file-level creation/overwrites, prefer `[log(json|yaml|toml)@path]{...}`.
   * Packets should be filtered through mod.rs for modularity purposes instead of imported directly.

4. ### **Safe Edits**

   * Append new packet modules under `src/packets/`.
   * Never overwrite `router.rs` wholesale — modify only within handlers.

5. ### **The Box Rule**

   * TagSpeak operates off of what is known as* **`red.tgsk`** *in its root.*  
    That file defines the lid to the **proverbial box.**  
    Everything TagSpeak — files, commands, network access —  
    to quote: **“Stays in the damn box.”**

    - If `red.tgsk` is missing → `E_BOX_REQUIRED`
    - If a path escapes the root → `E_BOX_VIOLATION`
    - Outside-world packets (`[exec]`, `[run]`, `[http]`) resolve against the box (paths) or are gated via `.tagspeak.toml` (network)
    - Default posture = **deny** unless explicitly allowed in `.tagspeak.toml`

6. ### Design Axioms

These are the guiding rules of TagSpeak. Codex should assume these are **always true** when interpreting or generating `.tgsk`.

1. **Everything is a Packet**  
   - Any computation, value, or action is represented as `[packet]`.  
   - Canonical max form: `[packet:label(conditional/argument)@value]`.  

2. **Human-Friendly + Machine-Precise**  
   - All packets must be readable for humans and trivially parsable for machines.  
   - Sugar exists for humans but always expands back to canonical form.  

3. **Dataflow > Control Flow**  
   - Execution flows along data passed between packets.  
   - Example: `[math@10+10]>[store@x]` = “result flows into `store`.”  

4. **Modular**  
   - Packets are interchangeable building blocks.  
   - Any packet can slot into any chain or nest without special casing.  

5. **Composable**  
   - Packets can chain (`>[next]`) and nest (`{ ... }`).  
   - Complex behavior emerges from layering simple packets.  

6. **Extensible**  
   - New packet types can be added without altering core grammar.  
   - Labels, arguments, and sugar extend capability safely.  

7. **Coexistence of Sugar + Canonical**  
   - Sugar forms (e.g. `[if(x==y)]{...}`) exist for readability.  
   - Canonical (`[cond(x==y)]>[then]{...}[else]{...}`) is always valid.  

8. **Readability Parity**  
   - Syntax must remain equally legible to humans and LLMs.  
   - Minimize boilerplate while preserving clarity.  

9. **Flow Around the User, Not the Language**  
   - Multiple valid forms are allowed.  
   - Tagspeak adapts to context; it does not force ceremony.  

10. **Inline Expansion is Truth**  
    - All sugar expands back to canonical packet form.  
    - No packet is “special.”  

11. **Minimal Boilerplate**  
    - Short, clear, unambiguous syntax is preferred.  
    - `[math@10+10]` is favored over verbose function calls.  

12. **LLM ↔ Human Accessibility**  
    - Domain is *shared understanding*.  
    - Syntax exists for fast parsing by both humans and AI.  

13. **Packets Define Behavior, Not Grammar**  
    - Behavior lives in packet modules.  
    - Grammar stays minimal and universal.  

14. **Conditionals as Dataflow**  
    - `[if(x==y)]{...}[else]{...}` routes values based on truth.  
    - No imperative branching, only flows.  

15. **Explicit State**  
    - Memory is always packetized (`[store]`, `[load]`, `[save]`, `[log]`).  
    - No hidden context.  

16. **Safety by Protocol Gating**  
    - Dangerous ops (`[exec]`) are color-gated (yellow/red).  
    - Consent is enforced at the syntax level.  

17. **Packet Additions**
    - Check suggested_packets first. Everytime.
    - Always verify whether a packet is already implemented before suggesting or creating it.
    - Packets must provide end-user value, not exist solely for repo debugging.
    - New packets must honor the core principles:
      - [everything_is_a_packet]
      - [sugar+canonical_coexist]
      - [readable_human+parsable_machine]
    - Do not propose duplicate or overlapping packets; prefer composition of existing ones.
    - Packet names should be short, intuitive, and descriptive of their behavior.
    - Each packet definition must specify:
      - What input(s) it accepts
      - What output it produces and passes downstream
    - Side-effect packets (e.g., [print], [log]) must still return/pass a value unless explicitly designed as sinks.
    - Packets that trigger external or system effects (file I/O, exec, network) must always be gated by explicit safety color `[yellow:]`
    - All new packets must include documentation:
      - At least one canonical example
      - Sugar example(s) if sugar form exists
      - Brief description of purpose and behavior






---

## Packet Status
Read \tagspeak_rs\docs\Tagspeak_101.md for a full list of added packets and usage.


### ✅ Implemented

* `[msg@"string"]` → create a string literal.
* `[int@int]` → create an integer.
* `[bool@true|false]` → create a bool value.
* `[note@"message"]` → inline documentation.
* `[math@expr]` → evaluate math expression.
* `[store@name]` → save last value under variable name.
* `[print]` → print the last value (or `[print@value]`).
* `[save@file]` → save current runtime state to file.
* `[load@file/path/relative/red.tgsk]` → load values/config from a file.
* `[log@file.json]` → dump last value to a JSON file (quick + dirty mode).
* `[mod@var]{...}` → edit an in-memory document. Operators:
  * `comp` → set value if absent.
  * `comp!` → overwrite existing value.
  * `merge` → deep merge object structures.
  * `del` → delete field/path.
  * `ins` → insert a new value.
  * Note for these: They are used within {...} of `[mod]`
* `[loopN]{...}` → repeat enclosed block. (allows for `[loopN@funct_name]`)
* `[if(condition)]{...}` → conditional execution if true.
* `[or(condition)]{...}` → else-if style branching.
* `[else]{...}` → fallback branch for conditionals.
* `[funct@name]{...}` → define a reusable function.
* `[log(json|yaml|toml)@file]{...}` → structured logging mode. 
  * `[key(name)@value]` → insert a key/value pair in a structured `[log]` block.
  * `[sect@section]{...}` → create a nested object/table (JSON/YAML/TOML style).
* `[call@funct_name] → call function directly
* `[exec@"cmd"]` → run shell command; returns stdout string.
  * Modes: `exec(code)`, `exec(stderr)`, `exec(json)`
* `[run@/path/script.tgsk]` → parse and execute another TagSpeak script within current runtime (respects red.tgsk root and updates cwd).
* `[yellow@"message"]{...}` / `[confirm@"message"]{...}` → prompt user before executing enclosed block. Env overrides:
  * `TAGSPEAK_ALLOW_YELLOW=1` approve all yellow prompts
  * `TAGSPEAK_ALLOW_EXEC=1` auto-approve `[exec]`
  * `TAGSPEAK_ALLOW_RUN=1` auto-approve `[run]` (default behavior already permissive)
* `[http(get|post|put|delete)@url]{ [key(header.Name)@val] [key(json)@{...}] [key(body)@"..."] }` → HTTP client (blocked unless enabled/allowlisted in `.tagspeak.toml`)
* `[parse(json|yaml|toml)@string]` → parse string into an in-memory document (usable with `[mod]`, `[dump]`)

### 🛠️ In Progress / Planned


---

## Do Not Touch

* **`router.rs`** — never overwrite wholesale. Only extend handlers.
* **Core runtime (`Runtime`, `Value`, `Packet`)** — do not redefine base types.
* **AST definitions (`ast.rs`)** — structural integrity must remain stable.

---

## Examples

### Quick Dump

```tgsk
[math@1+1]>[log@out.json]
```

Produces `out.json`: `2`

### JSON Structured Log

```tgsk
[log(json)@profile.json]{
  [key(name)@"Saryn"]
  [key(age)@25]
  [key(active)@true]
}
```

### YAML Structured Log

```tgsk
[log(yaml)@profile.yaml]{
  [key(name)@"Saryn"]
  [key(age)@25]
}
```

### TOML Structured Log with Sections

```tgsk
[log(toml)@Cargo.toml]{
  [sect@package]{
    [key(name)@"tagspeak"]
    [key(version)@"0.1.0"]
  }
  [sect@dependencies]{
    [key(anyhow)@"1"]
    [key(serde)@"1"]
  }
}
```

---