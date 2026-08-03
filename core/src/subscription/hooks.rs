use rquickjs::{Context, Error, Runtime};
use serde_json::{Value, json};

pub(super) fn run_generate_hook(source: &str, input: Value) -> Result<Value, String> {
    run_hook(source, "onGenerate", input)
}

pub fn run_finalize_hook(source: &str, config: Value) -> Result<Value, String> {
    run_hook(source, "onFinalize", json!({"singbox": config}))
}

fn run_hook(source: &str, name: &str, input: Value) -> Result<Value, String> {
    let runtime =
        Runtime::new().map_err(|e| format!("Failed to initialize Profile hook runtime: {e}"))?;
    let context = Context::full(&runtime)
        .map_err(|e| format!("Failed to initialize Profile hook context: {e}"))?;
    let input = serde_json::to_string(&input).map_err(|e| e.to_string())?;
    let script = format!(
        "{}\ntypeof {name} === 'function' ? JSON.stringify({name}({input})) : JSON.stringify(({input}).singbox)",
        source.replace("export ", "")
    );
    context
        .with(|ctx| {
            ctx.eval::<Option<String>, _>(script).map_err(|e| match e {
                Error::Exception => format!("Profile hook {name} execution failed"),
                _ => format!("Profile hook {name} execution failed: {e}"),
            })
        })
        .and_then(|value| {
            let value = value.ok_or_else(|| {
                format!("Profile hook {name} did not return a configuration object")
            })?;
            let value: Value = serde_json::from_str(&value)
                .map_err(|e| format!("Profile hook {name} returned invalid JSON: {e}"))?;
            if value.is_object() {
                Ok(value)
            } else {
                Err(format!(
                    "Profile hook {name} must return a configuration object"
                ))
            }
        })
}
