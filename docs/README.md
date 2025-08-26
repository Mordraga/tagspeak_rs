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
- [X] conditionals (`[if@(x>2)]{...}[else]{...}`)
- [X] modular imports / red.tgsk boundaries (everything under red.tgsk accessable)
- [x] Load JSON/YAML/TOML ([load@./file/path/relative/red.tgsk])

---

## Setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable recommended)
- Git (optional, for cloning the repository)

### Clone the Repository

```bash
git clone https://github.com/yourusername/tagspeak_rs.git
cd tagspeak_rs
```

### Build the Project

```bash
cargo build --release
```

### Run an Example Script

```bash
cargo run -- examples/smoke.tgsk
```

### Platform Setup

- **Windows:**  
  ```bash
  cargo run --bin tagspeak_setup
  ```
- **Linux:**  
  ```bash
  cargo run --bin tagspeak_setup_linux
  ```

### Running Tests

```bash
cargo test
```

### Notes

- Ensure a `red.tgsk` file exists in your project root (can be empty).
- Example scripts and data files are in the `examples/` directory.

---

## Contributing

Pull requests and issues are welcome! See `src/` for code organization.
```

---

### Examples

- See the `examples/` directory for sample `.tgsk` scripts and data files.
- Try:  
  ```bash
  cargo run -- examples/load_demo/load_yaml/main.tgsk
  ```
