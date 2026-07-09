use crate::clock_ms;
use wasm3::{Environment, Module};

pub fn wasm3_coremark(wasm: &[u8]) -> f32 {
    let env = Environment::new().expect("Wasm3: failed to create execution environment");
    let rt = env
        .create_runtime(2048)
        .expect("Wasm3: failed to create runtime");
    let mut module = rt
        .load_module(Module::parse(&env, wasm).expect("Wasm3: failed to parse Wasm module"))
        .expect("Wasm: failed to parse coremark Wasm module");
    module
        .link_closure::<(), u32, _>("env", "clock_ms", |_ctx, _args| Ok(clock_ms()))
        .expect("Wasm3: failed to link \"clock_ms\" function");
    module
        .find_function::<(), f32>("run")
        .expect("Wasm3: failed to find exported \"run\" function in Wasm module instance")
        .call()
        .expect("Wasm3: failed to call \"run\" function")
}
