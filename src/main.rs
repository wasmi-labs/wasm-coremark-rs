#[cfg(feature = "wasmi-v1")]
mod wasmi_v1;
#[cfg(feature = "wasmi")]
mod wasmi_v2;

#[cfg(feature = "wasmi-v1")]
use self::wasmi_v1::wasmi_v1_coremark;

#[cfg(feature = "wasmi")]
use self::wasmi_v2::wasmi_coremark;

#[cfg(any(feature = "wasmtime", feature = "winch"))]
use anyhow::Context as _;

// `spacewasm` is `no_std` and links its internal `Vec`/`Rc`/`InnerVec` against
// `extern "C"` allocation hooks (`__spacewasm_alloc` etc.) that the embedder
// must provide exactly once. This macro generates them — it is *not* Rust's
// `#[global_allocator]`, so it does not affect the other runtimes' allocations.
#[cfg(feature = "spacewasm")]
spacewasm::global_allocator!(
    spacewasm::PageAllocator<16>,
    spacewasm::PageAllocator::new(&spacewasm_util::RustSystemAllocator, 8192)
);

fn clock_ms() -> u32 {
    use std::time::Instant;
    static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed();
    elapsed.as_millis() as u32
}

pub enum WasmtimeBackend {
    Wasmtime,
    Winch,
    Pulley,
}

#[cfg(any(feature = "wasmtime", feature = "winch", feature = "pulley"))]
fn wasmtime_coremark_impl(backend: WasmtimeBackend, wasm: &[u8]) -> anyhow::Result<f32> {
    let mut config = wasmtime::Config::default();
    let strategy = match backend {
        WasmtimeBackend::Winch => wasmtime::Strategy::Winch,
        WasmtimeBackend::Pulley | WasmtimeBackend::Wasmtime => wasmtime::Strategy::Cranelift,
    };
    config.strategy(strategy);
    if matches!(backend, WasmtimeBackend::Pulley) {
        config
            .target("pulley64")
            .map_err(anyhow::Error::from)
            .context("failed to set target to `pulley64`")?;
    }
    let engine = wasmtime::Engine::new(&config)
        .map_err(anyhow::Error::from)
        .context("failed to create engine")?;
    let mut store = <wasmtime::Store<()>>::new(&engine, ());
    let mut linker = wasmtime::Linker::new(store.engine());
    linker
        .func_wrap("env", "clock_ms", clock_ms)
        .map_err(anyhow::Error::from)
        .context("failed to define `clock_ms` host function")?;
    let module = wasmtime::Module::new(store.engine(), wasm)
        .map_err(anyhow::Error::from)
        .context("failed to compile and validate coremark Wasm binary")?;
    let run = linker
        .instantiate(&mut store, &module)
        .map_err(anyhow::Error::from)
        .context("failed to instantiate coremark Wasm module")?
        .get_typed_func::<(), f32>(&mut store, "run")
        .map_err(anyhow::Error::from)
        .context("could not find \"run\" function export")?;
    let result = run
        .call(&mut store, ())
        .map_err(anyhow::Error::from)
        .context("failed to execute \"run\" function")?;
    Ok(result)
}

#[cfg(feature = "wasmtime")]
fn wasmtime_coremark(wasm: &[u8]) -> f32 {
    wasmtime_coremark_impl(WasmtimeBackend::Wasmtime, wasm)
        .context("Wasmtime")
        .unwrap()
}

#[cfg(feature = "winch")]
fn winch_coremark(wasm: &[u8]) -> f32 {
    wasmtime_coremark_impl(WasmtimeBackend::Winch, wasm)
        .context("Winch")
        .unwrap()
}

#[cfg(feature = "pulley")]
fn pulley_coremark(wasm: &[u8]) -> f32 {
    wasmtime_coremark_impl(WasmtimeBackend::Pulley, wasm)
        .context("Pulley")
        .unwrap()
}

#[cfg(feature = "stitch")]
fn stitch_coremark(wasm: &[u8]) -> f32 {
    use makepad_stitch as stitch;
    let engine = stitch::Engine::new();
    let mut store = <stitch::Store>::new(engine);
    let mut linker = stitch::Linker::new();
    linker.define("env", "clock_ms", stitch::Func::wrap(&mut store, clock_ms));
    let module = stitch::Module::new(store.engine(), wasm)
        .expect("Wasmi: failed to compile and validate coremark Wasm binary");
    let mut results = [stitch::Val::F32(0.0); 1];
    linker
        .instantiate(&mut store, &module)
        .expect("Wasmi: failed to start Wasm module instance")
        .exported_func("run")
        .expect("Wasmi: could not find \"run\" function export")
        .call(&mut store, &[], &mut results[..])
        .expect("Wasmi: failed to execute \"run\" function");
    let [result] = results;
    result.to_f32().unwrap()
}

#[cfg(feature = "wasm3")]
fn wasm3_coremark(wasm: &[u8]) -> f32 {
    use wasm3::{Environment, Module};
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

#[cfg(feature = "tinywasm")]
fn tinywasm_coremark(wasm: &[u8]) -> f32 {
    let mut store = tinywasm::Store::default();
    let mut imports = tinywasm::Imports::new();
    imports.define(
        "env",
        "clock_ms",
        tinywasm::HostFunction::from(&mut store, |_ctx, _arg: ()| Ok(clock_ms() as i32)),
    );
    let module = tinywasm::parse_bytes(wasm)
        .expect("Tinywasm: failed to compile and validate coremark Wasm binary");
    let instance = tinywasm::ModuleInstance::instantiate(&mut store, &module, Some(imports))
        .expect("Tinywasm: failed to instantiate Wasm module");
    instance
        .func::<(), f32>(&store, "run")
        .expect("Tinywasm: could not find \"run\" function export")
        .call(&mut store, ())
        .expect("Tinywasm: failed to execute \"run\" function")
}

#[cfg(feature = "wamr")]
fn wamr_coremark(wasm: &[u8]) -> f32 {
    use std::ffi::{CString, c_void};
    use std::ptr;
    use wamr_rust_sdk::{
        function::Function, instance::Instance, module::Module, runtime::Runtime, value::WasmValue,
    };

    extern "C" fn clock_ms_wamr(_exec_env: wamr_sys::wasm_exec_env_t) -> i32 {
        clock_ms() as i32
    }

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

#[cfg(feature = "wasmedge")]
fn wasmedge_coremark(wasm: &[u8]) -> f32 {
    use std::collections::HashMap;
    use wasmedge_sdk::{
        CallingFrame, FuncType, ImportObjectBuilder, Instance, Module, Store, ValType, Vm,
        WasmValue, error::CoreError, vm::SyncInst,
    };

    fn clock_ms_wasmedge(
        _data: &mut (),
        _inst: &mut Instance,
        _frame: &mut CallingFrame,
        _args: Vec<WasmValue>,
    ) -> Result<Vec<WasmValue>, CoreError> {
        Ok(vec![WasmValue::from_i32(clock_ms() as i32)])
    }

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

#[cfg(feature = "spacewasm")]
fn spacewasm_coremark(wasm: &[u8]) -> f32 {
    use core::ops::ControlFlow;
    use spacewasm::{
        CodeBuilder, CompilerOptions, ExportDesc, HostFunction, HostModule, InnerVec, Interpreter,
        InterpreterResult, InterpreterRunner, ModuleRef, RawValue, Ref, Store, Value, WasmRef,
        WasmStream,
    };

    // Feed the baked-in bytes straight to `spacewasm`'s streaming parser: hand
    // out `InnerVec` views into owned buffers we keep alive; `return_` is a
    // no-op (`InnerVec` has no `Drop`, so this is leak-only, which is fine for
    // a one-shot parse of the ~16 KiB coremark module).
    struct SliceStream<'a> {
        data: &'a [u8],
        pos: usize,
        bufs: Vec<Vec<u8>>,
    }
    impl WasmStream for SliceStream<'_> {
        fn read(&mut self) -> Result<Option<InnerVec<u8>>, u8> {
            if self.pos >= self.data.len() {
                return Ok(None);
            }
            let end = (self.pos + 1024).min(self.data.len());
            let mut buf = self.data[self.pos..end].to_vec();
            self.pos = end;
            let chunk = InnerVec {
                ptr: buf.as_mut_ptr(),
                capacity: buf.capacity() as u32,
                len: buf.len() as u32,
            };
            self.bufs.push(buf);
            Ok(Some(chunk))
        }

        fn return_(&mut self, _chunk: InnerVec<u8>) {}
    }

    const MAX_PAGES: usize = 32;
    const MAX_CONTROL_FRAMES: usize = 64;
    const MAX_STACK_DEPTH: usize = 256;

    let env = HostModule {
        name: "env",
        globals: spacewasm::vec![],
        functions: spacewasm::vec![HostFunction::new(
            "clock_ms",
            "".into(),
            // `'i'` is `i32` in spacewasm's signature DSL (`'I'` would be
            // `i64`); this coremark module imports `clock_ms: () -> i32`.
            "i".into(),
            |_, _| ControlFlow::Continue(Some(Value::I32(clock_ms() as i32))),
        )],
        memory: spacewasm::Vec::zero(),
        table: spacewasm::Vec::zero(),
    };

    let mut store = Store::new(2, [env]).expect("SpaceWasm: failed to create store");
    let mut code_builder = CodeBuilder::<MAX_PAGES>::default();
    let module = spacewasm::Module::new::<MAX_PAGES, MAX_CONTROL_FRAMES, MAX_STACK_DEPTH>(
        "coremark",
        &mut SliceStream {
            data: wasm,
            pos: 0,
            bufs: Vec::new(),
        },
        &mut store,
        &mut code_builder,
        spacewasm::Rc::new(spacewasm_util::RustSystemAllocator)
            .unwrap()
            .into_wasm_memory_allocator(),
        CompilerOptions::default(),
    )
    .expect("SpaceWasm: failed to compile and validate coremark Wasm binary");

    let (text, _) = code_builder.finish().unwrap();
    let mut state = store
        .allocate(1024)
        .expect("SpaceWasm: failed to allocate interpreter state");
    match state.initialize_module(module, &text, usize::MAX) {
        InterpreterResult::Finished => {}
        other => panic!("SpaceWasm: module initialization failed: {other:?}"),
    }

    // Resolve the exported `run` function.
    let module = state.store.modules().last().unwrap();
    let export = module
        .exports
        .iter()
        .find(|e| &e.name == "run")
        .expect("SpaceWasm: could not find \"run\" function export");
    let func = match export.desc {
        ExportDesc::Func(fi) => {
            let Ref::Module(fdi) = module.get_func_ref(fi).unwrap() else {
                panic!("SpaceWasm: invalid function ref")
            };
            WasmRef {
                module: ModuleRef(0),
                index: fdi,
            }
        }
        _ => panic!("SpaceWasm: \"run\" export is not a function"),
    };

    state
        .invoke(func, &[])
        .expect("SpaceWasm: failed to invoke \"run\"");
    let interpreter = Interpreter::default();
    let mut result = InterpreterResult::OutOfFuel;
    while result == InterpreterResult::OutOfFuel {
        result = interpreter.run(&text, &mut state, usize::MAX);
    }
    match result {
        InterpreterResult::Finished => {}
        other => panic!("SpaceWasm: failed to execute \"run\": {other:?}"),
    }
    state.result.unwrap_or(RawValue::from_32(0)).read_f32()
}

fn run_all(wasm: &[u8]) {
    type CoremarkRunner = fn(&[u8]) -> f32;
    let mut scores = Vec::new();
    let runtimes: [(&str, CoremarkRunner); _] = [
        #[cfg(feature = "wasmtime")]
        ("Wasmtime v46", wasmtime_coremark),
        #[cfg(feature = "winch")]
        ("Winch v46", winch_coremark),
        #[cfg(feature = "pulley")]
        ("Pulley v46", pulley_coremark),
        #[cfg(feature = "wasmi")]
        ("Wasmi v2", wasmi_coremark),
        #[cfg(feature = "wasmi-v1")]
        ("Wasmi v1", wasmi_v1_coremark),
        #[cfg(feature = "stitch")]
        ("Stitch", stitch_coremark),
        #[cfg(feature = "wasm3")]
        ("Wasm3", wasm3_coremark),
        #[cfg(feature = "tinywasm")]
        ("Tinywasm", tinywasm_coremark),
        #[cfg(feature = "wamr")]
        ("WAMR (fast interpreter)", wamr_coremark),
        #[cfg(feature = "wasmedge")]
        ("WasmEdge", wasmedge_coremark),
        #[cfg(feature = "spacewasm")]
        ("SpaceWasm", spacewasm_coremark),
    ];
    for (name, runtime) in runtimes {
        println!("Running Coremark using {name} ...");
        let score = runtime(wasm);
        scores.push((name, score));
        println!(" - Score: {score}\n")
    }
    println!("Scores");
    for (name, score) in scores {
        println!(" - {name}: \t{score}");
    }
}

#[allow(clippy::print_literal)]
fn help(args: &[String]) {
    println!(
        "Usage: {} [<WASM_RUNTIME>]\n\n{}\n{}\n{}\n{}",
        args[0],
        "WASM_RUNTIME: The WebAssembly runtime with which Coremark is run.",
        "              All available runners are used if the argument is not provided.",
        "",
        "              Possible Values: wasmtime, winch, pulley, wasmi, wasmi-v1, stitch, wasm3, tinywasm, wamr, wasmedge, spacewasm",
    )
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let coremark_wasm = include_bytes!("../coremark-minimal-mvp.wasm");
    match args.len() {
        1 => run_all(coremark_wasm),
        2 => {
            let engine = args[1].as_str();
            println!(
                "Running Coremark using {} ... [should take 12..20 seconds]",
                engine
            );
            let runtime = match engine {
                #[cfg(feature = "wasmtime")]
                "wasmtime" => wasmtime_coremark,
                #[cfg(feature = "winch")]
                "winch" => winch_coremark,
                #[cfg(feature = "pulley")]
                "pulley" => pulley_coremark,
                #[cfg(feature = "wasmi")]
                "wasmi" => wasmi_coremark,
                #[cfg(feature = "wasmi-v1")]
                "wasmi-v1" => wasmi_v1_coremark,
                #[cfg(feature = "stitch")]
                "stitch" => stitch_coremark,
                #[cfg(feature = "wasm3")]
                "wasm3" => wasm3_coremark,
                #[cfg(feature = "tinywasm")]
                "tinywasm" => tinywasm_coremark,
                #[cfg(feature = "wamr")]
                "wamr" => wamr_coremark,
                #[cfg(feature = "wasmedge")]
                "wasmedge" => wasmedge_coremark,
                #[cfg(feature = "spacewasm")]
                "spacewasm" => spacewasm_coremark,
                _ => return help(&args),
            };
            let score = runtime(coremark_wasm);
            println!(" - Score: {score}");
        }
        _ => help(&args),
    }
}
