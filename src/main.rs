use anyhow::{anyhow, Result};
use num_bigint::BigUint;
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

    // Instantiate the WASM module
    let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;


    // define the required calls into the wasm blob
    let from_montgomery = instance
        .get_export(&store, "ftm_fromMontgomery")
        .and_then(Extern::into_func)
        .ok_or_else(|| anyhow!("could not find function \"ftm_fromMontgomery\""))?
        .typed::<(i32, i32), ()>(&mut store)?;
    let to_montgomery = instance
        .get_export(&store, "ftm_toMontgomery")
        .and_then(Extern::into_func)
        .ok_or_else(|| anyhow!("could not find function \"ftm_toMontgomery\""))?
        .typed::<(i32, i32), ()>(&mut store)?;
    let compute_pairing = instance
        .get_export(&store, "bls12381_pairing")
        .and_then(Extern::into_func)
        .ok_or_else(|| anyhow!("could not find function \"bls12381_pairing\""))?
        .typed::<(i32, i32, i32), ()>(&mut store)?;

    // define the memory location of the G1, G2 generators in the WASM memory
    let p_g1: usize = 42104;
    let p_g2: usize = 42392;

    // obtain a copy of the WASM memory
    from_montgomery.call(&mut store, (p_g1 as i32, p_g1 as i32))?;
    let data: Vec<u8> = memory.data(&store).into_iter().copied().collect();
    to_montgomery.call(&mut store, (p_g1 as i32, p_g1 as i32))?;

    // Read G1 from WASM memory
    let g1 = [
        shift(p_g1, &data, 0),
        shift(p_g1, &data, 1),
        shift(p_g1, &data, 2),
    ];
    println!("g1: {:?}", g1);

    // Read G2 from WASM memory
    let g2 = [
        [[shift(p_g2, &data, 0), shift(p_g2, &data, 1)]],
        [[shift(p_g2, &data, 2), shift(p_g2, &data, 3)]],
        [[shift(p_g2, &data, 4), shift(p_g2, &data, 5)]],
    ];
    println!("g2: {:?}", g2);

    let g1_gen = [
        to_le("3685416753713387016781088315183077757961620795782546409894578378688607592378376318836054947676345821548104185464507"),
        to_le("1339506544944476473020471379941921221584933875938349620426543736416511423956333506472724655353366534992391756441569"),
        to_le("1")
    ];

    let g2_gen = [
        [to_le("352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160"),
        to_le("3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758")],
        [to_le("1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905"),
        to_le("927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582")],
        [to_le("1"),
        to_le("0")]
    ];

    println!("g1_gen: {:?}", g1_gen);
    println!("g2_gen: {:?}", g2_gen);

    // Define mmeory location to which we write the pairing result
    let p_res: usize = 98000;

    // compute the pairing and write it to p_res location in memory
    compute_pairing.call(&mut store, (p_g1 as i32, p_g2 as i32, p_res as i32))?;

    // get a copy of WASM memory
    from_montgomery.call(&mut store, (p_res as i32, p_res as i32))?;
    let data: Vec<u8> = memory.data(&store).into_iter().copied().collect();
    to_montgomery.call(&mut store, (p_res as i32, p_res as i32))?;

    // read data at result location
    let result = [
        [
            [from_le(shift(p_g1, &data, 0)), from_le(shift(p_g1, &data, 1))],
            [from_le(shift(p_g1, &data, 2)), from_le(shift(p_g1, &data, 3))],
            [from_le(shift(p_g1, &data, 4)), from_le(shift(p_g1, &data, 5))],
        ],
        [
            [from_le(shift(p_g1, &data, 6)), from_le(shift(p_g1, &data, 7))],
            [from_le(shift(p_g1, &data, 8)), from_le(shift(p_g1, &data, 9))],
            [from_le(shift(p_g1, &data, 10)), from_le(shift(p_g1, &data, 11))],
        ],
    ];
    println!("result: {:#?}", result);
    Ok(())
}

fn shift(start: usize, data: &Vec<u8>, pos: usize) -> Vec<u8> {
    let n8q: usize = 48;
    data.clone()[(start + pos * n8q)..(start + (pos + 1) * n8q - 1)].to_vec()
}

fn to_le(str: &str) -> Vec<u8> {
    BigUint::to_bytes_le(&str.parse::<BigUint>().unwrap())
}

fn from_le(vec: Vec<u8>) -> String {
    BigUint::from_bytes_le(&vec).to_string()
}
