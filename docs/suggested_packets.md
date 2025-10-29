**TagSpeak Flow Loops + Functions Spec**

> Codename: "Structured Flow"
> Purpose: Canonical definition for `[loop]`, `[funct]`, and related flow-control mechanics in TagSpeak.

---

## 🔁 Loop Packets

### `[loop@N]` — Finite Repetition

```tgsk
[loop@5]{ [call@step] }
```

* Runs enclosed block N times.
* Argument must be a literal number or variable.
* Breakable via `[break]`, `[return]`, or `[interrupt]`.

---

### `[loop:forever]` — Soft Infinite

```tgsk
[loop:forever]{ [call@tick] }
```

* Infinite loop with safe yield points.
* Auto-yields each iteration if inside `[async]`.
* Soft exit available via `[break]`, `[return]`, or `[interrupt]`.

---

### `[loop:until(condition)]` — Guarded Loop

```tgsk
[loop:until(condition)]{ [call@work] }
```

* Truthy → exits loop.
* Falsy → continues.
* Works with `[var@x]`, `[eq@...]`, or other condition packets.
* Working Example:
```
[funct:count_step]{
  [math@count+1]>[store@count]
  [print@count]
}

[int@0]>[store@count]

[loop:until@(count == 20)]{
  [call@count_step]
}
```

---

### `[loop:each(item@list)]` — Iteration

```tgsk
[loop:each(x@my_array)]{
  [print@x]
}
```

* Iterates over each item in the list.
* Sets `x` to the current item.
* Optional: `[loop:each(x, i@list)]` for index tracking.

---

## 🧠 Function Packets

### `[funct:tag]{...}` — Define Function

```tgsk
[funct:tick]{ [print@"tick"] }
```

* Creates a named, reusable block.
* Does not run until called.
* Can be async via `[fn(name):async]{}`.

---

### `[call@tag]` — Invoke Function

```tgsk
[call@tick]
```

* Calls previously defined function by tag.
* Passes current value in; returns function’s last value out.

---

### `[return]` — Exit Function or Loop

```tgsk
[return@42]
```

* Exits the current `[funct]` or `[loop]` early.
* Optional value returned.

---

### `[break]` / `[interrupt]`

* `[break]`: exits the current loop only.
* `[interrupt]`: exits loop **and** raises signal upstream (used to cascade break logic).

---

## 🧪 Flow Notes

* Loops can be nested.
* Functions can call other functions.
* Functions **can** contain loops.
* Use `[async]{ [loop] }` for time-based behavior.
* Loop guards (`[loop:until(...)]`) expect clean boolean context.

---

## ✅ Example

```tgsk
[funct:tick]{ [print@"tick"] }
[loop@3]{ [call@tick] }
```

---

## 🔧 Runtime Notes

* `[loop@N]` → for-loop.
* `[loop:forever]` → `loop { yield; }`
* `[loop:until]` → `while !cond { ... }`
* `[funct]` → registers block under tag.
* `[call]` → inlines or dispatches stored function.
* `[break]`, `[return]` are signal-passing control packets.

---
