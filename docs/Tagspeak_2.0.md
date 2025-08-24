### Everything is a Packet

Thoughts, code, memory, inputs, emotions — all exist in modular \[packet] form.
This includes files, functions, commands, and even syntax rules themselves.

## 🧠 Syntax Structure

**TagSpeak** follows a simplified English-like grammar:

### 📚 Subject → Object → Action

Each packet is modular and symbolic. Meaning is derived from tag type and position — not spacing or indentation.

---

### 📦 Basic Format

```
[subject@value]>[verb@modifier]>[action]
```

---

### 🔹 Example

```
[math@10+10]>[store@result]>[print]
```

> "Do math with 10 + 10, store the result, then print it."

This reads naturally and operates modularly:

* `[math@10+10]`: defines the source packet.
* `[store@result]`: routes the output to memory.
* `[print]`: invokes a return/display function.

---

### 🧰 Syntax Primitives

| Symbol  | Meaning                                    |
| ------- | ------------------------------------------ |
| `[...]` | Single packet                              |
| `@`     | Denotes input to the packet                |
| `>`     | Output / routing between packets           |
| `:`     | Used for nested logic (optional)           |
| `->`    | Symbolic flow inside a packet (pure logic) |

---

TagSpeak is intentionally **visually parseable** — for AIs, for humans, for scripts. Everything is a packet. Everything flows.

## 🔀 Conditionals

Use packets to express branching logic:

```
[if@(expr)]>[then]{ ... }>[or@(expr)]>[then]{ ... }>[else]{ ... }
```

* `[if@(expr)]` – evaluate the boolean expression.
* `[or@(expr)]` – optional additional branches; each acts as an `else if`.
* `[else]` – final fallback when no condition matched.

Boolean expressions support both tag-style and symbolic operators:

| Tag     | Symbol | Meaning |
| ------- | ------ | ------- |
| `[eq]`  | `==`   | equals  |
| `[neq]` | `!=`   | not equals |
| `[lt]`  | `<`    | less than |
| `[gt]`  | `>`    | greater than |
| `[and]` | `&&`   | logical and |
| `[or]`  | `||`   | logical or |
| `[not]` | `!`    | logical not |

Example:

```
[math@1] > [store@x]

[if@(x [eq] 2)]
[then]{ [print@"eq branch"] }
[or@(x == 1)]
[then]{ [print@"double equals branch"] }
[else]{ [print@"else branch"] }
```

The script above prints `double equals branch`.
