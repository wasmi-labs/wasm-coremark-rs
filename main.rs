#[cfg(any(feature = "wasmtime", feature = "winch"))]
use anyhow::Context as _;

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

#[cfg(feature = "wasmi")]
fn wasmi_coremark(wasm: &[u8]) -> f32 {
    let mut store = <wasmi::Store<()>>::default();
    let mut linker = wasmi::Linker::new(store.engine());
    linker
        .func_wrap("env", "clock_ms", clock_ms)
        .expect("Wasmi: failed to define `clock_ms` host function");
    let module = wasmi::Module::new(store.engine(), wasm)
        .expect("Wasmi: failed to compile and validate coremark Wasm binary");
    linker
        .instantiate_and_start(&mut store, &module)
        .expect("Wasmi: failed to start Wasm module instance")
        .get_typed_func::<(), f32>(&mut store, "run")
        .expect("Wasmi: could not find \"run\" function export")
        .call(&mut store, ())
        .expect("Wasmi: failed to execute \"run\" function")
}

#[cfg(feature = "wasmi-v1")]
fn wasmi_v1_coremark(wasm: &[u8]) -> f32 {
    use wasmi_v1 as wasmi;
    let mut store = <wasmi::Store<()>>::default();
    let mut linker = wasmi::Linker::new(store.engine());
    linker
        .func_wrap("env", "clock_ms", clock_ms)
        .expect("Wasmi: failed to define `clock_ms` host function");
    let module = wasmi::Module::new(store.engine(), wasm)
        .expect("Wasmi: failed to compile and validate coremark Wasm binary");
    linker
        .instantiate_and_start(&mut store, &module)
        .expect("Wasmi: failed to start Wasm module instance")
        .get_typed_func::<(), f32>(&mut store, "run")
        .expect("Wasmi: could not find \"run\" function export")
        .call(&mut store, ())
        .expect("Wasmi: failed to execute \"run\" function")
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

fn run_all(wasm: &[u8]) {
    type CoremarkRunner = fn(&[u8]) -> f32;
    let mut scores = Vec::new();
    let runtimes: [(&str, CoremarkRunner); _] = [
        #[cfg(feature = "wasmtime")]
        ("Wasmtime v45", wasmtime_coremark),
        #[cfg(feature = "winch")]
        ("Winch v45", winch_coremark),
        #[cfg(feature = "pulley")]
        ("Pulley v45", pulley_coremark),
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
        "              Possible Values: wasmtime, winch, pulley, wasmi, wasmi-v1, stitch, wasm3, tinywasm",
    )
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let coremark_wasm = include_bytes!("coremark-minimal.wasm");
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
                _ => return help(&args),
            };
            let score = runtime(coremark_wasm);
            println!(" - Score: {score}");
        }
        _ => help(&args),
    }
}
