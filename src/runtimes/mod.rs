#[cfg(feature = "spacewasm")]
mod spacewasm;
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
pub use self::wasmi_v1::wasmi_v1_coremark;

#[cfg(feature = "wasmi")]
pub use self::wasmi_v2::wasmi_coremark;

#[cfg(feature = "wasmtime")]
pub use self::wasmtime::wasmtime_coremark;

#[cfg(feature = "winch")]
pub use self::wasmtime::winch_coremark;

#[cfg(feature = "pulley")]
pub use self::wasmtime::pulley_coremark;

#[cfg(feature = "stitch")]
pub use self::stitch::stitch_coremark;

#[cfg(feature = "wasm3")]
pub use self::wasm3::wasm3_coremark;

#[cfg(feature = "tinywasm")]
pub use self::tinywasm::tinywasm_coremark;

#[cfg(feature = "wamr")]
pub use self::wamr::wamr_coremark;

#[cfg(feature = "wasmedge")]
pub use self::wasmedge::wasmedge_coremark;

#[cfg(feature = "spacewasm")]
pub use self::spacewasm::spacewasm_coremark;
