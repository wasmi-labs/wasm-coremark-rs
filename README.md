# WASMi Coremark

This repo provides script for running the [coremark-minimal.wasm][0] using 
wasmtime and wasmi.

## Usage

```
usage: bm [wasmitime|wasm3|wasmi: string] [times: number]
```

## CoreMark

### Results (Apple M2 Pro)

| Runtime  | Version         | Score |
|:---------|----------------:|------:|
| Wasmtime | `v45`           | 30686 |
| Wasm3    | `v0.5`          |  2919 |
| Wasmi    | `v2.0.0-beta.3` |  2382 |
| Stitch   | `v0.1`          |  2223 |
| Wasmi    | `v1.0.9`        |  1972 |

The `coremark-minimal.wasm` we are using here does not produce text output like [coremark][1], just the final test result. 

## LICENSE

MIT

[0]: https://github.com/wasm3/wasm-coremark
[1]: https://github.com/eembc/coremark#log-file-format
