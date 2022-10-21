use anyhow::{anyhow, Result};
use num_bigint::BigUint;
use wasmi::{Engine, Extern, Instance, Linker, Memory, MemoryType, Module, Store};

const P_G1: i32 = 42104;
const P_G2: i32 = 42392;

struct WasmInstance {
    memory: Memory,
    instance: Instance,
    store: Store<i32>,
}

impl WasmInstance {
    fn from_file(path: &str) -> Result<Self, anyhow::Error> {
        // First step is to create the Wasm execution engine with some config.
        // In this example we are using the default configuration.
        let engine = Engine::default();
        let wasm = std::fs::read(path)
            .map_err(|_| anyhow!("failed to read Wasm file"))
            .unwrap();
        let module = Module::new(&engine, &mut &wasm[..])?;

        // All Wasm objects operate within the context of a `Store`.
        // Each `Store` has a type parameter to store host-specific data,
        // which in this case we are using `42` for.
        type HostState = u32;
        let mut store = Store::new(&engine, 42);

        // In order to create Wasm module instances and link their imports
        // and exports we require a `Linker`.
        let mut linker = <Linker<HostState>>::new();

        let memory = Memory::new(&mut store, MemoryType::new(30, None))
            .map_err(|_| anyhow!("failed to define WASM memory"))?;
        linker.define("env", "memory", memory)?;
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;
        Ok(Self {
            memory,
            instance,
            store,
        })
    }

    fn from_montgomery(&mut self, from: i32, to: i32) {
        self.instance
            .get_export(&self.store, "ftm_fromMontgomery")
            .and_then(Extern::into_func)
            .ok_or_else(|| anyhow!("could not find function \"ftm_fromMontgomery\""))
            .unwrap()
            .typed::<(i32, i32), ()>(&mut self.store)
            .unwrap()
            .call(&mut self.store, (from, to))
            .unwrap();
    }

    fn to_montgomery(&mut self, from: i32, to: i32) {
        self.instance
            .get_export(&self.store, "ftm_toMontgomery")
            .and_then(Extern::into_func)
            .ok_or_else(|| anyhow!("could not find function \"ftm_toMontgomery\""))
            .unwrap()
            .typed::<(i32, i32), ()>(&mut self.store)
            .unwrap()
            .call(&mut self.store, (from, to))
            .unwrap();
    }

    fn compute_pairing(&mut self, p_g1: i32, p_g2: i32, p_res: i32) {
        self.instance
            .get_export(&self.store, "bls12381_pairing")
            .and_then(Extern::into_func)
            .ok_or_else(|| anyhow!("could not find function \"bls12381_pairing\""))
            .unwrap()
            .typed::<(i32, i32, i32), ()>(&mut self.store)
            .unwrap()
            .call(&mut self.store, (p_g1, p_g2, p_res))
            .unwrap();
    }

    fn g1m_neg(&mut self, from: i32, to: i32) {
        self.instance
            .get_export(&self.store, "g1m_neg")
            .and_then(Extern::into_func)
            .ok_or_else(|| anyhow!("could not find function \"g1m_neg\""))
            .unwrap()
            .typed::<(i32, i32), ()>(&mut self.store)
            .unwrap()
            .call(&mut self.store, (from, to))
            .unwrap();
    }

    fn ftm_conjugate(&mut self, from: i32, to: i32) {
        self.instance
            .get_export(&self.store, "ftm_conjugate")
            .and_then(Extern::into_func)
            .ok_or_else(|| anyhow!("could not find function \"ftm_conjugate\""))
            .unwrap()
            .typed::<(i32, i32), ()>(&mut self.store)
            .unwrap()
            .call(&mut self.store, (from, to))
            .unwrap();
    }

    fn g2m_neg(&mut self, from: i32, to: i32) {
        self.instance
            .get_export(&self.store, "g2m_neg")
            .and_then(Extern::into_func)
            .ok_or_else(|| anyhow!("could not find function \"g2m_neg\""))
            .unwrap()
            .typed::<(i32, i32), ()>(&mut self.store)
            .unwrap()
            .call(&mut self.store, (from, to))
            .unwrap();
    }

    fn get_f12(&mut self, p_f12: i32, in_montgomery: bool) -> [[[BigUint; 2]; 3]; 2] {
        if !in_montgomery {
            self.from_montgomery(p_f12, p_f12);
        }
        let data: Vec<u8> = self.memory.data(&self.store).to_vec();
        if !in_montgomery {
            self.to_montgomery(p_f12, p_f12)
        };
        let p_f12 = p_f12 as usize;
        [
            [
                [
                    from_le(shift(p_f12, &data, 0)),
                    from_le(shift(p_f12, &data, 1)),
                ],
                [
                    from_le(shift(p_f12, &data, 2)),
                    from_le(shift(p_f12, &data, 3)),
                ],
                [
                    from_le(shift(p_f12, &data, 4)),
                    from_le(shift(p_f12, &data, 5)),
                ],
            ],
            [
                [
                    from_le(shift(p_f12, &data, 6)),
                    from_le(shift(p_f12, &data, 7)),
                ],
                [
                    from_le(shift(p_f12, &data, 8)),
                    from_le(shift(p_f12, &data, 9)),
                ],
                [
                    from_le(shift(p_f12, &data, 10)),
                    from_le(shift(p_f12, &data, 11)),
                ],
            ],
        ]
    }

    fn g1(&self) -> [Vec<u8>; 3] {
        let data: Vec<u8> = self.memory.data(&self.store).to_vec();
        [
            shift(P_G1 as usize, &data, 0),
            shift(P_G1 as usize, &data, 1),
            shift(P_G1 as usize, &data, 2),
        ]
    }

    fn g2(&self) -> [[[Vec<u8>; 2]; 1]; 3] {
        let p_g2 = P_G2 as usize;
        let data: Vec<u8> = self.memory.data(&self.store).to_vec();
        [
            [[shift(p_g2, &data, 0), shift(p_g2, &data, 1)]],
            [[shift(p_g2, &data, 2), shift(p_g2, &data, 3)]],
            [[shift(p_g2, &data, 4), shift(p_g2, &data, 5)]],
        ]
    }
}

fn shift(start: usize, data: &[u8], pos: usize) -> Vec<u8> {
    let n8q: usize = 48;
    data[(start + pos * n8q)..(start + (pos + 1) * n8q)].to_vec()
}

fn to_le(str: &str) -> Vec<u8> {
    BigUint::to_bytes_le(&str.parse::<BigUint>().unwrap())
}

fn from_le(vec: Vec<u8>) -> BigUint {
    BigUint::from_bytes_le(&vec)
}

fn main() -> Result<()> {
    let mut wasm = WasmInstance::from_file("bls12381.wasm")?;
    let p_result: i32 = 127000;
    wasm.compute_pairing(P_G1, P_G2, p_result);
    let result = wasm.get_f12(p_result, false);
    println!("result: {:?}", result);
    let result_montgomery = wasm.get_f12(p_result, true);
    println!("result in montgomery: {:?}", result_montgomery);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paring_is_unitary() {
        let mut wasm =
            WasmInstance::from_file("bls12381.wasm").expect("Failed to instantiate WASM");

        // Define memory location to which we write the computation results
        let p_n_g1: i32 = 125000;
        let p_n_g2: i32 = 126000;
        let p_p: i32 = 127000;
        let p_q: i32 = 128000;
        let p_r: i32 = 129000;

        // compute the pairing and write it to p_res location in memory
        wasm.g1m_neg(P_G1, p_n_g1);
        wasm.g2m_neg(P_G2, p_n_g2);
        wasm.compute_pairing(P_G1, P_G2, p_p);
        wasm.ftm_conjugate(p_p, p_p);
        wasm.compute_pairing(p_n_g1, P_G2, p_q);
        wasm.compute_pairing(P_G1, p_n_g2, p_r);
        let p = wasm.get_f12(p_p, false);
        let q = wasm.get_f12(p_q, false);
        let r = wasm.get_f12(p_r, false);
        assert_eq!(p, q);
        assert_eq!(q, r);
    }
}
