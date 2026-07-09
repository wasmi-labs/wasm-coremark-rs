use std::ffi::{CString, c_void};
use std::ptr;
use wamr_rust_sdk::{
    function::Function, instance::Instance, module::Module, runtime::Runtime, value::WasmValue,
};
use crate::clock_ms;

extern "C" fn clock_ms_wamr(_exec_env: wamr_sys::wasm_exec_env_t) -> i32 {
    clock_ms() as i32
}

pub fn wamr_coremark(wasm: &[u8]) -> f32 {
    // `wamr-rust-sdk`'s builder only registers host functions under the module
    // name "host", but coremark imports `env::clock_ms`, so register it here
    // directly. WAMR keeps the pointers to `symbols`, `symbol_name` and
    // `module_name`, so they are declared before `runtime` to outlive it.
    let symbol_name = CString::new("clock_ms").unwrap();
    let module_name = CString::new("env").unwrap();
    let mut symbols = [wamr_sys::NativeSymbol {
        symbol: symbol_name.as_ptr(),
        func_ptr: clock_ms_wamr as *mut c_void,
        signature: ptr::null(),
        attachment: ptr::null_mut(),
    }];

    let runtime = Runtime::new().expect("WAMR: failed to create runtime");
    let registered = unsafe {
        wamr_sys::wasm_runtime_register_natives(
            module_name.as_ptr(),
            symbols.as_mut_ptr(),
            symbols.len() as u32,
        )
    };
    assert!(
        registered,
        "WAMR: failed to register `env::clock_ms` host function"
    );

    let module = Module::from_vec(&runtime, wasm.to_vec(), "coremark")
        .expect("WAMR: failed to compile and validate coremark Wasm binary");
    let instance = Instance::new(&runtime, &module, 64 * 1024)
        .expect("WAMR: failed to instantiate Wasm module");
    let result = Function::find_export_func(&instance, "run")
        .expect("WAMR: could not find \"run\" function export")
        .call(&instance, &Vec::new())
        .expect("WAMR: failed to execute \"run\" function");
    match result.as_slice() {
        [WasmValue::F32(score)] => *score,
        other => panic!("WAMR: expected a single f32 result, got {other:?}"),
    }
}
