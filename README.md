# WebAssembly Coremark

This repo provides script for running the [coremark-minimal.wasm][0] using a variety of Wasm runtimes.

## Usage

```
Usage: target/debug/bm [<WASM_RUNTIME>]

WASM_RUNTIME: The WebAssembly runtime with which Coremark is run.
              All available runners are used if the argument is not provided.

              Possible Values: wasmtime, winch, pulley, wasmi, wasmi-v1, stitch, wasm3, tinywasm, wamr, wasmedge
```

## Create Features

The Wasmtime, WAMR and WasmEdge dependencies are disabled by default to keep compiled times down.  
Enable Wasmtime via `wasmtime`, Winch via `winch` and Pulley via `pulley` crate feature

**Example:** `cargo run --release -F wasmtime`

## CoreMark

### Scores (Apple M2 Pro)

| Runtime            | Version         | Type        | Score |
|:-------------------|----------------:|:-----------:|------:|
| Wasmtime Cranelift | `v46`           | JIT         | 30086 |
| Wasmtime Winch     | `v46`           | JIT         | 13598 |
| Wasm3              | `v0.5`          | Interpreter |  2825 |
| Wasmi v2           | `v2.0.0-beta.5` | Interpreter |  2802 |
| Stitch             | `v0.1`          | Interpreter |  2241 |
| Wasmi v1           | `v1.0.9`        | Interpreter |  2031 |
| Wasmtime Pulley    | `v46`           | Interpreter |  1786 |
| WAMR (fast)        | `v2.3`          | Interpreter |  1420 |
| Tinywasm           | `v0.9`          | Interpreter |   937 |
| WasmEdge           | `v0.14`         | Interpreter |   313 |

### Scores (Wasmi Configs)

| `portable-dispatch` | `indirect-dispatch` | Score |
|:-------------------:|:-------------------:|------:|
| ❌                  | ❌                  |  2802 |
| ❌                  | ✅                  |  2357 |
| ✅                  | ❌                  |  1133 |
| ✅                  | ✅                  |  1764 |

The `coremark-minimal.wasm` we are using here does not produce text output like [coremark][1], just the final test result. 

## LICENSE

MIT

[0]: https://github.com/wasm3/wasm-coremark
[1]: https://github.com/eembc/coremark#log-file-format
