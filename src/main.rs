use anyhow::{anyhow, Result};
use wasmi::{Engine, Extern, Linker, Memory, MemoryType, Module, Store};

fn main() -> Result<()> {
    // First step is to create the Wasm execution engine with some config.
    // In this example we are using the default configuration.
    let engine = Engine::default();
    let wasm = std::fs::read("bls12381.wasm").map_err(|_| anyhow!("failed to read Wasm file"))?;

    let module = Module::new(&engine, &mut &wasm[..])?;

    // All Wasm objects operate within the context of a `Store`.
    // Each `Store` has a type parameter to store host-specific data,
    // which in this case we are using `42` for.
    type HostState = u32;
    let mut store = Store::new(&engine, 42);

    // In order to create Wasm module instances and link their imports
    // and exports we require a `Linker`.
    let mut linker = <Linker<HostState>>::new();
    // Instantiation of a Wasm module requires defining its imports and then
    // afterwards we can fetch exports by name, as well as asserting the
    // type signature of the function with `get_typed_func`.
    //

    // Instantiate memory object and link it:
    let memory = Memory::new(&mut store, MemoryType::new(25, None));
    let memory = memory.unwrap();
    linker.define("env", "memory", memory)?;

    // Instantiate
    let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

    // Export WASM call for pairing
    let compute_pairing = instance
        .get_export(&store, "bls12381_pairing")
        .and_then(Extern::into_func)
        .ok_or_else(|| anyhow!("could not find function \"bls12381_pairing\""))?
        .typed::<(i32, i32, i32), ()>(&mut store)?;

    compute_pairing.call(&mut store, (42104, 42392, 92000))?;

    // export WASM call for conversion from Montgomery
    let from_montgomery = instance
        .get_export(&store, "ftm_fromMontgomery")
        .and_then(Extern::into_func)
        .ok_or_else(|| anyhow!("could not find function \"ftm_fromMontgomery\""))?
        .typed::<(i32, i32), ()>(&mut store)?;
        from_montgomery.call(&mut store, (92000, 92000))?;

    // read data at result location
    let data = memory.data(&store).clone();
    let result = [
        [
            [shift(data, 0), shift(data, 1)],
            [shift(data, 2), shift(data, 3)],
            [shift(data, 4), shift(data, 5)],
        ],
        [
            [shift(data, 6), shift(data, 7)],
            [shift(data, 8), shift(data, 9)],
            [shift(data, 10), shift(data, 11)],
        ],
    ];
    println!("result: {:?}", result);
    Ok(())
}

fn shift(data: &[u8], pos: usize) -> &[u8] {
    let start: usize = 92000;
    let n8q: usize = 96;
    &data[(start + pos * n8q * 5)..92000 + ((start + 1) * 5*  n8q - 1)]
}
