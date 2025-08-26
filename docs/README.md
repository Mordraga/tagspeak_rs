# TagSpeak RS

TagSpeak is a symbolic, packet-based language designed to be **human-readable** and **machine-parsable**.  
This Rust implementation (`tagspeak_rs`) provides an interpreter that can parse and execute `.tgsk` scripts.

---

## ✨ Core Ideas
- **Everything is a packet** → `[op@arg]`
- **Packets can chain** with `>` → `[math@2+2] > [print@result]`
- **Blocks** use `{ ... }` → group multiple packets
- **Strings** use quotes → `[print@"hello world"]`
- **Comments** supported → `#`, `//`, `/* ... */`

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

---

## 📦 Example `.tgsk`

```tgsk
// comments are fine
[note@"Init counter"]
[math@0] > [store@counter] > [print@counter]

[note@"Inline loop"]
[loop@3]{ [math@counter+1] > [store@counter] > [print@counter] }

[note@"Tag loop"]
[funct:step]{ [math@counter+1] > [store@counter] > [print@counter] }
[math@0] > [store@counter] > [print@counter]
[loop3@step]
```

---

## 🚀 Run

```bash
cargo run -- examples/smoke.tgsk
```

Expected output:

```
0
1
2
3
0
1
2
3
```

---

## 🛣 Roadmap
- [x] math/store/print/note
- [x] funct + loop (inline + tag)
- [ ] call tags directly (`[call@step]`)
- [x] conditionals (`[if@([a][gt][b]>{[...]}>[...])]`)
- [ ] modular imports / red.tgsk boundaries
