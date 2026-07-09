use crate::clock_ms;
use makepad_stitch::{Engine, Func, Linker, Module, Store, Val};

pub fn stitch_coremark(wasm: &[u8]) -> f32 {
    let engine = Engine::new();
    let mut store = <Store>::new(engine);
    let mut linker = Linker::new();
    linker.define("env", "clock_ms", Func::wrap(&mut store, clock_ms));
    let module = Module::new(store.engine(), wasm)
        .expect("Stitch: failed to compile and validate coremark Wasm binary");
    let mut results = [Val::F32(0.0); 1];
    linker
        .instantiate(&mut store, &module)
        .expect("Stitch: failed to start Wasm module instance")
        .exported_func("run")
        .expect("Stitch: could not find \"run\" function export")
        .call(&mut store, &[], &mut results[..])
        .expect("Stitch: failed to execute \"run\" function");
    let [result] = results;
    result.to_f32().unwrap()
}
