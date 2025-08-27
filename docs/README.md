# TagSpeak RS

TagSpeak is a symbolic, packet-based language designed to be **human-readable** and **machine-parsable**.  
This Rust implementation (`tagspeak_rs`) provides an interpreter that can parse and execute `.tgsk` scripts.

---

## ✨ Core Ideas
- **Everything is a packet** → `[op@arg]`
- **Packets can chain** with `>` → `[math@2+2] > [print@result]`
- **Blocks** use `{ ... }` → group multiple packets
- **Strings** use quotes → `[print@"hello world"]`
- **Comments** supported → `#`, `//`, `/* ... */` or tagspeak's own `[note@]`

---

## 🔧 Features Implemented
- **math** → evaluate expressions with `meval`
- **store** → assign variables
- **print** → output values or strings
- **note** → dev/debug annotation
- **funct** → define named blocks
- **loop** → two styles:
  - `[loop@3]{ ... }` → inline loop
  - `[funct:step]{ ... } … [loop3@step]` → tag loop (modular, reusable)
- **load** → load JSON/YAML/TOML files **relative to the nearest `red.tgsk`**  
  (`[load@./file/path/relative/to/red.tgsk]`)
- **red.tgsk** → Root file marker/sentinel file. Must exist in your project root; all file access is sandboxed to this boundary.

...

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
- [ ] call tags directly (`[call@step]`)
- [ ] conditionals (`[if@(x>2)]{...}[else]{...}`)
- [ ] modular imports / red.tgsk boundaries
