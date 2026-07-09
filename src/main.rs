mod runtimes;

use self::runtimes::*;

fn clock_ms() -> u32 {
    use std::time::Instant;
    static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed();
    elapsed.as_millis() as u32
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
