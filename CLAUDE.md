# Done

A TypeScript runtime with predictable memory behavior. Built as a learning project, with a real thesis: JS developers working on latency-sensitive workloads (HFT bots, game servers, real-time systems) have no good option today. Node/Deno/Bun all use V8, which has a non-deterministic major GC that can pause for 10-100ms at any time. Done uses QuickJS, which uses reference counting — objects are freed immediately and synchronously when the last reference drops. No stop-the-world pauses.

## The thesis

V8's GC is a tracing GC running on background threads with write barriers. Even with concurrent marking, you can't eliminate pauses — you need a consistent heap snapshot, which requires stopping the world briefly. Ref-counting sidesteps this entirely: no tracing, no background threads, no write barriers, no pauses. The only exception is the cycle collector (for objects that reference each other), which Done exposes as a manual `gc()` call so you can trigger it at a safe point of your choosing.

## Memory model

- **Ref-counting (free):** QuickJS does this by default. When the last reference to an object drops, it's freed immediately and synchronously on the main thread.
- **Cycle collector (manual):** QuickJS's cycle collector handles reference cycles (A → B → A). Done exposes this as `gc()` so you choose when it runs.
- **`using` blocks (planned):** TypeScript 5.2's `using` keyword calls `Symbol.dispose` at end of scope. Combined with ref-counting, this gives you deterministic cleanup at a specific line of code.
- **Arena allocation (future):** QuickJS exposes `JSMallocFunctions` — a pluggable C-level allocator. Long-term goal: allocate objects into an arena, free the whole slab at once.

## Stack

- **Language:** Rust
- **JS Engine:** QuickJS via `rquickjs`
- **Transpiler:** SWC (`swc_core`) — strips TypeScript types before execution
- **Input:** TypeScript
- **Target:** Predictable-latency TypeScript runtime

## Execution pipeline

```
.ts file → SWC (strip types) → JS string → QuickJS (execute)
```

## Project structure

```
done/
├── src/
│   ├── main.rs        # Entry point
│   ├── runtime.rs     # Core runtime, engine setup, built-in globals
│   └── transpiler.rs  # SWC-based TypeScript → JavaScript
├── ts/                # Test TypeScript files
├── Cargo.toml
└── CLAUDE.md
```

## Milestones

- [x] Embed QuickJS, execute a TypeScript file
- [x] console.log/warn/error with object stringification
- [x] Object.prototype.toString returns JSON
- [x] Expose `gc()` global — triggers QuickJS cycle collector via `Runtime::run_gc()`
- [x] Expose `memoryUsage()` global — surfaces `Runtime::memory_usage()`
- [ ] `using` block support — SWC already parses it; wire up `Symbol.dispose`
- [x] Basic event loop
- [ ] File I/O (read/write)
- [ ] Module resolution
- [ ] Arena allocation via `JSMallocFunctions` (requires C-level QuickJS fork)

## Conventions

- No unnecessary abstraction — if it doesn't need to be a trait, it isn't
- Errors handled explicitly, no unwrap() in production paths
- Keep it readable over clever
