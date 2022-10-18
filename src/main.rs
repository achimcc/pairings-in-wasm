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
    linker.define("env", "memory", memory.unwrap())?;

    // Instantiate
    let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;
     
    // Export WASM ccall
    let square = instance
        .get_export(&store, "f2m_square")
        .and_then(Extern::into_func)
        .ok_or_else(|| anyhow!("could not find function \"f2m_square\""))?
        .typed::<(i32, i32), ()>(&mut store)?;
    // And finally we can call the wasm!

    square.call(&mut store, (5, 4))?;
    Ok(())
}
