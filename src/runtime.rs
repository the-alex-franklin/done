use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::{function::Rest, Context, Ctx, Function, Object, Runtime, Value};

pub fn run(js: &str) -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;

    ctx.with(|ctx| -> rquickjs::Result<()> {
        setup_console(&ctx)?;
        setup_gc(&ctx)?;
        setup_timers(&ctx)?;
        ctx.eval::<(), _>(
            "Object.prototype.toString = function() { return JSON.stringify(this); };",
        )?;
        ctx.eval::<(), _>(js)?;
        run_event_loop(&ctx)?;
        Ok(())
    })?;

    Ok(())
}

// ------ event loop ------

struct Timer<'js> {
    fire_at: Instant,
    id: u32,
    interval_ms: Option<u64>,
    callback: Function<'js>,
}

thread_local! {
    static NEXT_ID: RefCell<u32> = const { RefCell::new(1) };
}

fn next_id() -> u32 {
    NEXT_ID.with(|c| {
        let id = *c.borrow();
        *c.borrow_mut() = id.wrapping_add(1);
        id
    })
}

fn setup_timers<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let queue: Rc<RefCell<Vec<Timer<'js>>>> = Rc::new(RefCell::new(Vec::new()));
    let cancelled: Rc<RefCell<HashSet<u32>>> = Rc::new(RefCell::new(HashSet::new()));

    // Store both in the context so run_event_loop can retrieve them.
    // We use a JS global holding an opaque Rust pointer — simplest approach
    // without unsafe is to just clone the Rc into each closure.
    let queue_set = Rc::clone(&queue);
    let queue_interval = Rc::clone(&queue);
    let cancelled_clear = Rc::clone(&cancelled);

    ctx.globals().set(
        "__timerQueue__",
        Function::new(ctx.clone(), {
            let ctx2 = ctx.clone();
            move |cb: Function<'js>, ms: u64, repeat: bool| -> u32 {
                let id = next_id();
                queue_set.borrow_mut().push(Timer {
                    fire_at: Instant::now() + Duration::from_millis(ms),
                    id,
                    interval_ms: repeat.then_some(ms),
                    callback: cb,
                });
                let _ = ctx2; // keep ctx alive in closure
                id
            }
        })?,
    )?;

    ctx.globals().set(
        "__cancelTimer__",
        Function::new(ctx.clone(), move |id: u32| {
            cancelled_clear.borrow_mut().insert(id);
        })?,
    )?;

    // Expose the Rc to run_event_loop via thread_local so we can get at it
    // without unsafe pointer games. Store them in thread-locals keyed to this
    // invocation. run_event_loop reads and clears them.
    TIMER_QUEUE.with(|tq| {
        // SAFETY: we extend the lifetime to 'static for storage; we clear this
        // before ctx.with() returns, so the queue never outlives 'js.
        let queue_static: Rc<RefCell<Vec<Timer<'static>>>> =
            unsafe { std::mem::transmute(queue) };
        let cancelled_static: Rc<RefCell<HashSet<u32>>> = cancelled;
        *tq.borrow_mut() = Some((queue_static, cancelled_static));
    });

    ctx.eval::<(), _>(
        r#"
        function setTimeout(cb, ms) {
            ms = ms || 0;
            return __timerQueue__(cb, ms, false);
        }
        function clearTimeout(id) { __cancelTimer__(id); }
        function setInterval(cb, ms) {
            ms = ms || 0;
            return __timerQueue__(cb, ms, true);
        }
        function clearInterval(id) { __cancelTimer__(id); }
        "#,
    )?;

    // Also register the interval re-queue helper — JS side just calls
    // __timerQueue__ again via setInterval, so nothing extra needed here.
    //
    // Queue the interval re-registration inside the Rust event loop below.

    let _ = queue_interval; // moved into thread_local via transmute above
    Ok(())
}

thread_local! {
    static TIMER_QUEUE: RefCell<Option<(
        Rc<RefCell<Vec<Timer<'static>>>>,
        Rc<RefCell<HashSet<u32>>>,
    )>> = const { RefCell::new(None) };
}

fn run_event_loop<'js>(_ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let (queue, cancelled) = TIMER_QUEUE.with(|tq| tq.borrow_mut().take())
        .expect("event loop state missing");

    // Re-attach the correct lifetime.
    let queue: Rc<RefCell<Vec<Timer<'js>>>> = unsafe { std::mem::transmute(queue) };

    loop {
        let next_fire = {
            let q = queue.borrow();
            if q.is_empty() {
                break;
            }
            q.iter().map(|t| t.fire_at).min().unwrap()
        };

        let now = Instant::now();
        if next_fire > now {
            std::thread::sleep(next_fire - now);
        }

        let now = Instant::now();
        let fired: Vec<Timer<'js>> = {
            let mut q = queue.borrow_mut();
            let mut fired = Vec::new();
            let mut remaining = Vec::new();
            for t in q.drain(..) {
                if t.fire_at <= now {
                    fired.push(t);
                } else {
                    remaining.push(t);
                }
            }
            *q = remaining;
            fired
        };

        for t in fired {
            if cancelled.borrow().contains(&t.id) {
                continue;
            }
            t.callback.call::<(), ()>(())?;
            if let Some(ms) = t.interval_ms {
                // Re-queue — but only if not cancelled during the callback.
                if !cancelled.borrow().contains(&t.id) {
                    queue.borrow_mut().push(Timer {
                        fire_at: Instant::now() + Duration::from_millis(ms),
                        id: t.id,
                        interval_ms: Some(ms),
                        callback: t.callback,
                    });
                }
            }
        }
    }

    Ok(())
}

// ------ gc / memory ------

fn setup_gc<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    globals.set(
        "gc",
        Function::new(ctx.clone(), |ctx: Ctx<'_>| {
            unsafe {
                let rt = rquickjs::qjs::JS_GetRuntime(ctx.as_raw().as_ptr());
                rquickjs::qjs::JS_RunGC(rt);
            }
        })?,
    )?;

    globals.set(
        "memoryUsage",
        Function::new(ctx.clone(), |ctx: Ctx<'js>| -> rquickjs::Result<Object<'js>> {
            let mut m = rquickjs::qjs::JSMemoryUsage {
                malloc_size: 0, malloc_limit: 0, memory_used_size: 0,
                malloc_count: 0, memory_used_count: 0, atom_count: 0,
                atom_size: 0, str_count: 0, str_size: 0, obj_count: 0,
                obj_size: 0, prop_count: 0, prop_size: 0, shape_count: 0,
                shape_size: 0, js_func_count: 0, js_func_size: 0,
                js_func_code_size: 0, js_func_pc2line_count: 0,
                js_func_pc2line_size: 0, c_func_count: 0, array_count: 0,
                fast_array_count: 0, fast_array_elements: 0,
                binary_object_count: 0, binary_object_size: 0,
            };
            unsafe {
                let rt = rquickjs::qjs::JS_GetRuntime(ctx.as_raw().as_ptr());
                rquickjs::qjs::JS_ComputeMemoryUsage(rt, &mut m);
            }
            let obj = Object::new(ctx)?;
            obj.set("mallocSize", m.malloc_size)?;
            obj.set("mallocLimit", m.malloc_limit)?;
            obj.set("memoryUsedSize", m.memory_used_size)?;
            obj.set("mallocCount", m.malloc_count)?;
            obj.set("objCount", m.obj_count)?;
            obj.set("objSize", m.obj_size)?;
            obj.set("strCount", m.str_count)?;
            obj.set("strSize", m.str_size)?;
            obj.set("jsFuncCount", m.js_func_count)?;
            Ok(obj)
        })?,
    )?;

    Ok(())
}

// ------ console ------

fn setup_console<'js>(ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    let console = Object::new(ctx.clone())?;
    console.set("log", Function::new(ctx.clone(), js_log)?)?;
    console.set("error", Function::new(ctx.clone(), js_error)?)?;
    console.set("warn", Function::new(ctx.clone(), js_warn)?)?;
    globals.set("console", console)?;
    Ok(())
}

fn js_log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<()> {
    println!("{}", format_args_list(&ctx, &args.0));
    Ok(())
}

fn js_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<()> {
    eprintln!("{}", format_args_list(&ctx, &args.0));
    Ok(())
}

fn js_warn<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<()> {
    eprintln!("warn: {}", format_args_list(&ctx, &args.0));
    Ok(())
}

fn format_args_list<'js>(ctx: &Ctx<'js>, args: &[Value<'js>]) -> String {
    args.iter().map(|v| format_value(ctx, v)).collect::<Vec<_>>().join(" ")
}

fn format_value<'js>(ctx: &Ctx<'js>, v: &Value<'js>) -> String {
    if let Some(s) = v.as_string() {
        s.to_string().unwrap_or_default()
    } else if v.is_null() {
        "null".to_owned()
    } else if v.is_undefined() {
        "undefined".to_owned()
    } else if let Some(b) = v.as_bool() {
        b.to_string()
    } else if let Some(i) = v.as_int() {
        i.to_string()
    } else if let Some(f) = v.as_float() {
        f.to_string()
    } else {
        ctx.json_stringify(v.clone())
            .ok()
            .flatten()
            .and_then(|s| s.to_string().ok())
            .unwrap_or_else(|| "[object Object]".to_owned())
    }
}
