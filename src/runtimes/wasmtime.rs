use crate::clock_ms;
use anyhow::Context as _;
use anyhow::Error;
use wasmtime::{Config, Engine, Linker, Module, Store, Strategy};

pub enum WasmtimeBackend {
    Wasmtime,
    Winch,
    Pulley,
}

#[cfg(any(feature = "wasmtime", feature = "winch", feature = "pulley"))]
fn wasmtime_coremark_impl(backend: WasmtimeBackend, wasm: &[u8]) -> anyhow::Result<f32> {
    let mut config = Config::default();
    let strategy = match backend {
        WasmtimeBackend::Winch => Strategy::Winch,
        WasmtimeBackend::Pulley | WasmtimeBackend::Wasmtime => Strategy::Cranelift,
    };
    config.strategy(strategy);
    if matches!(backend, WasmtimeBackend::Pulley) {
        config
            .target("pulley64")
            .map_err(Error::from)
            .context("failed to set target to `pulley64`")?;
    }
    let engine = Engine::new(&config)
        .map_err(Error::from)
        .context("failed to create engine")?;
    let mut store = <Store<()>>::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    linker
        .func_wrap("env", "clock_ms", clock_ms)
        .map_err(Error::from)
        .context("failed to define `clock_ms` host function")?;
    let module = Module::new(store.engine(), wasm)
        .map_err(Error::from)
        .context("failed to compile and validate coremark Wasm binary")?;
    let run = linker
        .instantiate(&mut store, &module)
        .map_err(Error::from)
        .context("failed to instantiate coremark Wasm module")?
        .get_typed_func::<(), f32>(&mut store, "run")
        .map_err(Error::from)
        .context("could not find \"run\" function export")?;
    let result = run
        .call(&mut store, ())
        .map_err(Error::from)
        .context("failed to execute \"run\" function")?;
    Ok(result)
}

#[cfg(feature = "wasmtime")]
pub fn wasmtime_coremark(wasm: &[u8]) -> f32 {
    wasmtime_coremark_impl(WasmtimeBackend::Wasmtime, wasm)
        .context("Wasmtime")
        .unwrap()
}

#[cfg(feature = "winch")]
pub fn winch_coremark(wasm: &[u8]) -> f32 {
    wasmtime_coremark_impl(WasmtimeBackend::Winch, wasm)
        .context("Winch")
        .unwrap()
}

#[cfg(feature = "pulley")]
pub fn pulley_coremark(wasm: &[u8]) -> f32 {
    wasmtime_coremark_impl(WasmtimeBackend::Pulley, wasm)
        .context("Pulley")
        .unwrap()
}
