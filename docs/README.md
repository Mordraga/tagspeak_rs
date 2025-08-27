# TagSpeak RS

TagSpeak is a symbolic, packet-based language designed to be **human-readable** and **machine-parsable**.
This Rust implementation (`tagspeak_rs`) provides an interpreter for `.tgsk` scripts. See [TagSpeak 101](Tagspeak_101.md) for a quick reference.

---

## ✨ Core Ideas
- **Everything is a packet** → `[op@arg]`
- **Packets can chain** with `>` → `[math@2+2] > [print@result]`
- **Blocks** use `{ ... }` → group multiple packets
- **Strings** use quotes → `[print@"hello world"]`
- **Comments** supported → `#`, `//`, `/* ... */` or tagspeak's own `[note@]`

---

## 🔧 Features Implemented
- **math** → evaluate expressions
- **store** → assign variables
- **print** → output values or strings
- **note** → inline documentation
- **funct** → define reusable blocks
- **call** → invoke functions (`[call@name]`)
- **loop** → `[loop3]{ ... }` or `[loop3@funct]`
- **if/or/else** → conditional branching
- **load** → read JSON/YAML/TOML based on file extension
- **save** → persist runtime state
- **log** → structured file logging (`[log(json|yaml|toml)@file]{...}`)
- **mod** → edit in-memory documents
- **red.tgsk** → sentinel file marking the project root for file access

### Notes

- Ensure a `red.tgsk` file exists in your project root (can be empty).
- All `[load@...]` paths are resolved relative to the nearest `red.tgsk`—files outside this boundary cannot be accessed.
- Example scripts and data files are in the `examples/` directory.

---

## Using `.tgsk`

### 🚀 Run

```bash
cargo run -- examples/smoke.tgsk
```

### Testing

Unit tests are included for core packets.
To run tests:
```bash
cargo test
```
---

## 🛣 Roadmap
- [x] math/store/print/note
- [x] funct + loop (inline + tag)
- [x] call tags directly (`[call@step]`)
- [x] conditionals (`[if@(x>2)]{...}[else]{...}`)
- [ ] modular imports / red.tgsk boundaries
