use rquickjs::{function::Rest, Context, Ctx, Function, Object, Runtime, Value};

pub fn run(js: &str) -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;

    ctx.with(|ctx| -> rquickjs::Result<()> {
        setup_console(&ctx)?;
        ctx.eval::<(), _>(
            "Object.prototype.toString = function() { return JSON.stringify(this); };",
        )?;
        ctx.eval::<(), _>(js)?;
        Ok(())
    })?;

    Ok(())
}

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
