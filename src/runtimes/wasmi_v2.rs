use crate::clock_ms;
use wasmi::{Linker, Module, Store};

pub fn wasmi_coremark(wasm: &[u8]) -> f32 {
    let mut store = <Store<()>>::default();
    let mut linker = Linker::new(store.engine());
    linker
        .func_wrap("env", "clock_ms", clock_ms)
        .expect("Wasmi: failed to define `clock_ms` host function");
    let module = Module::new(store.engine(), wasm)
        .expect("Wasmi: failed to compile and validate coremark Wasm binary");
    linker
        .instantiate_and_start(&mut store, &module)
        .expect("Wasmi: failed to start Wasm module instance")
        .get_typed_func::<(), f32>(&mut store, "run")
        .expect("Wasmi: could not find \"run\" function export")
        .call(&mut store, ())
        .expect("Wasmi: failed to execute \"run\" function")
}
