# Done

A TypeScript runtime built for latency-sensitive workloads. Node, Deno, and Bun all run on V8, which has a tracing GC that can stop the world for 10–100ms at unpredictable times. Done runs on QuickJS, which uses reference counting — memory is freed immediately when the last reference drops. No background GC threads. No stop-the-world pauses.

Built as a learning project, but the thesis is real.

## The problem

If you're writing an HFT bot, a game server, or any real-time system in TypeScript, you're at the mercy of V8's GC. Even with concurrent marking, V8 has to stop the world briefly to get a consistent heap snapshot. You can tune it, hint at it, and work around it — but you can't eliminate it.

Reference counting sidesteps this entirely. When the last reference to an object drops, it's freed right there, synchronously, on the main thread. No tracing, no background threads, no write barriers. The only exception is reference cycles (A → B → A), which ref-counting can't handle alone — Done exposes `gc()` to trigger QuickJS's cycle collector manually, so you choose when it runs.

## Usage

```
cargo install --path .
```

> **Note:** `done` is a reserved word in bash/zsh (it closes `do` loops). Call it by full path or alias it:
> ```
> alias done='~/.cargo/bin/done'
> ```

Then run any `.ts` file:

```
~/.cargo/bin/done script.ts
```

## What's built in

**Memory**
```ts
gc();                // trigger the cycle collector — call it at a safe point
memoryUsage();       // returns { objCount, mallocSize, memoryUsedSize, ... }
```

**Deterministic cleanup with `using`**
```ts
class DbConnection {
  // @ts-ignore
  [Symbol.dispose]() { this.close(); }
}

function handleRequest() {
  using db = new DbConnection();
  // ...
  // db.close() called here automatically, at this exact line
}
```

TypeScript 5.2's `using` keyword calls `Symbol.dispose` at end of scope. Combined with ref-counting, cleanup happens at a specific, predictable point in your code.

**Timers**
```ts
setTimeout(() => {}, 100);
setInterval(() => {}, 30);
clearTimeout(id);
clearInterval(id);
```

**File I/O**
```ts
fs.writeFileSync("out.txt", "hello");
const data = fs.readFileSync("out.txt");
fs.appendFileSync("out.txt", " world");
fs.existsSync("out.txt");   // true
fs.unlinkSync("out.txt");
```

**Console**
```ts
console.log("hello", { a: 1 });   // objects printed as JSON
console.warn("watch out");
console.error("something broke");
```

## Stack

| Layer | What |
|---|---|
| Language | Rust |
| JS engine | QuickJS via [rquickjs](https://github.com/DelSkayn/rquickjs) |
| Transpiler | SWC — strips TypeScript types, lowers `using` blocks |
| Input | TypeScript |

## How it works

```
.ts file → SWC (strip types + lower using blocks) → JS string → QuickJS (execute)
```

SWC handles TypeScript → JavaScript. QuickJS executes the result. The Rust layer wires them together and exposes built-in globals.

## What's next

- [ ] Module resolution (`import` / `require`)
- [ ] Arena allocation via `JSMallocFunctions` — allocate into a slab, free the whole thing at once (requires a C-level QuickJS fork)
