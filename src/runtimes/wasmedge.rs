use crate::clock_ms;
use std::collections::HashMap;
use wasmedge_sdk::{
    CallingFrame, FuncType, ImportObjectBuilder, Instance, Module, Store, ValType, Vm, WasmValue,
    error::CoreError, vm::SyncInst,
};

fn clock_ms_wasmedge(
    _data: &mut (),
    _inst: &mut Instance,
    _frame: &mut CallingFrame,
    _args: Vec<WasmValue>,
) -> Result<Vec<WasmValue>, CoreError> {
    Ok(vec![WasmValue::from_i32(clock_ms() as i32)])
}

pub fn wasmedge_coremark(wasm: &[u8]) -> f32 {
    // The host import must resolve under the module name `env`, which is the
    // import object's own name (the `Store` map key only tracks the borrow).
    let mut import = ImportObjectBuilder::<()>::new("env", ())
        .expect("WasmEdge: failed to create import object");
    import
        .with_func_by_type(
            "clock_ms",
            FuncType::new(vec![], vec![ValType::I32]),
            clock_ms_wasmedge,
        )
        .expect("WasmEdge: failed to define `clock_ms` host function");
    let mut import = import.build();

    let mut instances: HashMap<String, &mut dyn SyncInst> = HashMap::new();
    instances.insert("env".to_string(), &mut import);
    let store = Store::new(None, instances).expect("WasmEdge: failed to create store");
    let mut vm = Vm::new(store);

    let module = Module::from_bytes(None, wasm)
        .expect("WasmEdge: failed to compile and validate coremark Wasm binary");
    vm.register_module(None, module)
        .expect("WasmEdge: failed to instantiate Wasm module");
    vm.run_func(None, "run", Vec::<WasmValue>::new())
        .expect("WasmEdge: failed to execute \"run\" function")[0]
        .to_f32()
}
