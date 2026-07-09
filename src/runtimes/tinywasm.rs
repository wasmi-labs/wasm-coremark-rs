use crate::clock_ms;
use tinywasm::{HostFunction, Imports, ModuleInstance, Store};

pub fn tinywasm_coremark(wasm: &[u8]) -> f32 {
    let mut store = Store::default();
    let mut imports = Imports::new();
    imports.define(
        "env",
        "clock_ms",
        HostFunction::from(&mut store, |_ctx, _arg: ()| Ok(clock_ms() as i32)),
    );
    let module = tinywasm::parse_bytes(wasm)
        .expect("Tinywasm: failed to compile and validate coremark Wasm binary");
    let instance = ModuleInstance::instantiate(&mut store, &module, Some(imports))
        .expect("Tinywasm: failed to instantiate Wasm module");
    instance
        .func::<(), f32>(&store, "run")
        .expect("Tinywasm: could not find \"run\" function export")
        .call(&mut store, ())
        .expect("Tinywasm: failed to execute \"run\" function")
}
