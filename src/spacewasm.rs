use crate::clock_ms;
use core::ops::ControlFlow;
use spacewasm::{
    CodeBuilder, CompilerOptions, ExportDesc, HostFunction, HostModule, InnerVec, Interpreter,
    InterpreterResult, InterpreterRunner, Module, ModuleRef, PageAllocator, RawValue, Rc, Ref,
    Store, Value, WasmRef, WasmStream,
};
use spacewasm_util::RustSystemAllocator;

// `spacewasm` is `no_std` and links its internal `Vec`/`Rc`/`InnerVec` against
// `extern "C"` allocation hooks (`__spacewasm_alloc` etc.) that the embedder
// must provide exactly once. This macro generates them — it is *not* Rust's
// `#[global_allocator]`, so it does not affect the other runtimes' allocations.
spacewasm::global_allocator!(
    PageAllocator<16>,
    PageAllocator::new(&RustSystemAllocator, 8192)
);

const MAX_PAGES: usize = 32;
const MAX_CONTROL_FRAMES: usize = 64;
const MAX_STACK_DEPTH: usize = 256;

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

pub fn spacewasm_coremark(wasm: &[u8]) -> f32 {
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
    let module = Module::new::<MAX_PAGES, MAX_CONTROL_FRAMES, MAX_STACK_DEPTH>(
        "coremark",
        &mut SliceStream {
            data: wasm,
            pos: 0,
            bufs: Vec::new(),
        },
        &mut store,
        &mut code_builder,
        Rc::new(RustSystemAllocator)
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
