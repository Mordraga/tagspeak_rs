# TagSpeak 101

Welcome to TagSpeak — a symbolic mini-language where **everything is a packet**.

---

## 🧩 What’s a Packet?
The basic unit is `[op@arg]`.  
Think of it as: **verb + data**.

Examples:
- `[math@2+2]`
- `[store@counter]`
- `[print@"hello"]`

Packets **chain** with `>`:
```tgsk
[math@2+2] > [store@x] > [print@x]
```

---

## 🔲 Blocks
Use `{ ... }` to group packets into a sequence.

```tgsk
[loop@3]{ 
  [print@"hi"] 
}
```

---

## 🗒 Comments
TagSpeak allows 3 styles:
```tgsk
# hash
// double slash
/* block comment */
```

---

## ⚙️ Built-in Packets

### `[math@expr]`
Evaluate an expression.  
```tgsk
[math@5*2] > [print]
```

### `[store@var]`
Assigns the last value to `var`.  
```tgsk
[math@10] > [store@x]
```

### `[print@val]`
Output either a value or string.  
```tgsk
[print@"hello world"]
```

### `[note@"..."]`
Inline annotation; printed in dev/debug mode.

### `[funct:tag]{ ... }`
Define a named block of code.  
```tgsk
[funct:step]{ [print@"inside step"] }
```

### `[loop@N]{ ... }`
Run the block N times (inline loop).

### `[loopN@tag]`
Run a named block N times.  
```tgsk
[funct:step]{ [print@"hi"] }
[loop3@step]
```

---

## 🚦 Design Rules
1. **Everything is a packet.**
2. **User-dependent flow** → syntax has sugar and modularity; you choose.
3. **Readable to humans, parseable by machines.**

---

## 🛠 Try It
Create `examples/hello.tgsk`:

```tgsk
[print@"Hello, TagSpeak!"]
```

Run:
```bash
cargo run -- examples/hello.tgsk
```
