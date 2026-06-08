# WASMi Coremark

This repo provides script for running the [coremark-minimal.wasm][0] using 
wasmtime and wasmi.

## Usage

```
usage: bm [wasmitime|wasm3|wasmi|wasmi-v1|stitch: string] [times: number]
```

## CoreMark

### Scores (Apple M2 Pro)

| Runtime  | Version         | Type        | Score |
|:---------|----------------:|:-----------:|------:|
| Wasmtime | `v45`           | JIT         | 30686 |
| Winch    | `v45`           | JIT         | 12846 |
| Wasm3    | `v0.5`          | Interpreter |  2919 |
| Wasmi    | `v2.0.0-beta.3` | Interpreter |  2382 |
| Stitch   | `v0.1`          | Interpreter |  2223 |
| Wasmi    | `v1.0.9`        | Interpreter |  1972 |
| Pulley   | `v45`           | Interpreter |  1710 |

The `coremark-minimal.wasm` we are using here does not produce text output like [coremark][1], just the final test result. 

## LICENSE

MIT

[0]: https://github.com/wasm3/wasm-coremark
[1]: https://github.com/eembc/coremark#log-file-format
