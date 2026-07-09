#[cfg(feature = "stitch")]
mod stitch;
#[cfg(feature = "tinywasm")]
mod tinywasm;
#[cfg(feature = "wamr")]
mod wamr;
#[cfg(feature = "wasm3")]
mod wasm3;
#[cfg(feature = "wasmedge")]
mod wasmedge;
#[cfg(feature = "wasmi-v1")]
mod wasmi_v1;
#[cfg(feature = "wasmi")]
mod wasmi_v2;
#[cfg(any(feature = "wasmtime", feature = "winch", feature = "pulley"))]
mod wasmtime;

#[cfg(feature = "wasmi-v1")]
use self::wasmi_v1::wasmi_v1_coremark;

#[cfg(feature = "wasmi")]
use self::wasmi_v2::wasmi_coremark;

#[cfg(feature = "wasmtime")]
use self::wasmtime::wasmtime_coremark;

#[cfg(feature = "winch")]
use self::wasmtime::winch_coremark;

#[cfg(feature = "pulley")]
use self::wasmtime::pulley_coremark;

#[cfg(feature = "stitch")]
use self::stitch::stitch_coremark;

#[cfg(feature = "wasm3")]
use self::wasm3::wasm3_coremark;

#[cfg(feature = "tinywasm")]
use self::tinywasm::tinywasm_coremark;

#[cfg(feature = "wamr")]
use self::wamr::wamr_coremark;

#[cfg(feature = "wasmedge")]
use self::wasmedge::wasmedge_coremark;

fn clock_ms() -> u32 {
    use std::time::Instant;
    static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed();
    elapsed.as_millis() as u32
}

// `spacewasm` is `no_std` and links its internal `Vec`/`Rc`/`InnerVec` against
// `extern "C"` allocation hooks (`__spacewasm_alloc` etc.) that the embedder
// must provide exactly once. This macro generates them — it is *not* Rust's
// `#[global_allocator]`, so it does not affect the other runtimes' allocations.
#[cfg(feature = "spacewasm")]
spacewasm::global_allocator!(
    spacewasm::PageAllocator<16>,
    spacewasm::PageAllocator::new(&spacewasm_util::RustSystemAllocator, 8192)
);

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
