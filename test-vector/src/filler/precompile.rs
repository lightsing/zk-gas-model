use crate::{
    TestCaseBuilder, TestCaseKind,
    filler::{StackBuilder, default_bytecode_with_pop_builder},
};
use ark_ff::Field;
use evm_guest::*;
use itertools::Itertools;
use rand::Rng;
use revm_precompile::u64_to_address;
use std::{
    collections::BTreeMap,
    ops::{Neg, Sub},
    sync::Arc,
};

const PRECOMPILE_CALL_MAX_GAS: u64 = u32::MAX as u64;

pub(crate) fn fill(map: &mut BTreeMap<Address, Arc<TestCaseBuilder>>) {
    fill_ec_add(map);
    fill_ec_mul(map);
    fill_ec_pair(map);
}

fn fill_ec_add(map: &mut BTreeMap<Address, Arc<TestCaseBuilder>>) {
    use bn128::*;

    let addr = u64_to_address(0x06);

    map.insert(
        addr,
        Arc::new(TestCaseBuilder {
            description: Arc::from("ecAdd"),
            kind: TestCaseKind::ConstantMixed,
            support_repetition: 1..1024 / OpCode::STATICCALL.inputs() as usize,
            memory_builder: Box::new(|memory, params| {
                let mut rng = params.rng();

                let memory_size = params.repetition * 32 * 4;
                memory.resize(memory_size);
                let mut context_memory_mut = memory.context_memory_mut();
                let mut buffer = context_memory_mut.as_mut();

                for _ in 0..params.repetition {
                    write_g1(&mut buffer[..G1_LEN], &rand_g1(&mut rng));
                    write_g1(&mut buffer[G1_LEN..], &rand_g1(&mut rng));
                    run_add(
                        &buffer[..ADD_INPUT_LEN],
                        ADD_GAS_COST,
                        PRECOMPILE_CALL_MAX_GAS,
                    )
                    .unwrap();
                    buffer = &mut buffer[ADD_INPUT_LEN..];
                }
            }),
            stack_builder: call_stack_builder(addr, ADD_INPUT_LEN, PRECOMPILE_CALL_MAX_GAS),
            bytecode_builder: default_bytecode_with_pop_builder(OpCode::STATICCALL),
            ..Default::default()
        }),
    );
}

fn fill_ec_mul(map: &mut BTreeMap<Address, Arc<TestCaseBuilder>>) {
    use bn128::*;

    let addr = u64_to_address(0x07);

    map.insert(
        addr,
        Arc::new(TestCaseBuilder {
            description: Arc::from("ecMul"),
            kind: TestCaseKind::DynamicMixed,
            support_repetition: 1..1024 / OpCode::STATICCALL.inputs() as usize,
            support_input_size: (0..254).collect(), // how many 1 in the scalar bits
            memory_builder: Box::new(|memory, params| {
                let mut rng = params.rng();

                let memory_size = params.repetition * MUL_INPUT_LEN;
                memory.resize(memory_size);
                let mut context_memory_mut = memory.context_memory_mut();
                let mut buffer = context_memory_mut.as_mut();

                // scalar = 2^input_size - 1
                // we won't exceed the 254 bits of Fr
                let scalar = U256::from(2u8)
                    .pow(U256::from(params.input_size as u32))
                    .sub(U256::ONE)
                    .to_be_bytes::<32>();

                for _ in 0..params.repetition {
                    write_g1(&mut buffer[..G1_LEN], &rand_g1(&mut rng));
                    buffer[G1_LEN..G1_LEN + SCALAR_LEN].copy_from_slice(&scalar);
                    run_mul(
                        &buffer[..MUL_INPUT_LEN],
                        MUL_GAS_COST,
                        PRECOMPILE_CALL_MAX_GAS,
                    )
                    .unwrap();
                    buffer = &mut buffer[MUL_INPUT_LEN..];
                }
            }),
            stack_builder: call_stack_builder(addr, MUL_INPUT_LEN, PRECOMPILE_CALL_MAX_GAS),
            bytecode_builder: default_bytecode_with_pop_builder(OpCode::STATICCALL),
            ..Default::default()
        }),
    );
}

fn fill_ec_pair(map: &mut BTreeMap<Address, Arc<TestCaseBuilder>>) {
    use bn128::*;

    let addr = u64_to_address(0x08);

    const BLOCK_GAS_TARGET: u64 = 20_000_000;
    const MAX_PAIR_LEN: u64 = (BLOCK_GAS_TARGET - PAIR_BASE_COST) / PAIR_PER_POINT_COST;

    map.insert(
        addr,
        Arc::new(TestCaseBuilder {
            description: Arc::from("ecPairing"),
            kind: TestCaseKind::DynamicMixed,
            support_repetition: 1..1024 / OpCode::STATICCALL.inputs() as usize,
            support_input_size: (2..MAX_PAIR_LEN as usize).collect(),
            memory_builder: Box::new(|memory, params| {
                let mut rng = params.rng();

                let memory_size = params.repetition * params.input_size * PAIR_ELEMENT_LEN;
                memory.resize(memory_size);
                let mut context_memory_mut = memory.context_memory_mut();
                let mut buffer = context_memory_mut.as_mut();

                for _ in 0..params.repetition {
                    let p = rand_g1(&mut rng);
                    let q = rand_g2(&mut rng);

                    let n = params.input_size - 1;
                    let coeffs = (&mut rng)
                        .random_iter::<u64>()
                        .take(n)
                        .map(|a| Fr::from(a))
                        .collect_vec();

                    // e(a*P, (a^-1)*Q)} = e(P, Q) ^ (a * a^-1) = e(P, Q)
                    for (i, a) in coeffs.into_iter().enumerate() {
                        let a_inv = a.inverse().unwrap();
                        let p = G1Affine::from(p * a);
                        let q = G2Affine::from(q * a_inv);
                        write_g1(&mut buffer[PAIR_ELEMENT_LEN * i..], &p);
                        write_g2(&mut buffer[PAIR_ELEMENT_LEN * i + G1_LEN..], &q);
                    }

                    let last_coeff = Fr::from(n as u32).neg();
                    let last_p = G1Affine::from(p * last_coeff);
                    write_g1(
                        &mut buffer[PAIR_ELEMENT_LEN * (params.input_size - 1)..],
                        &last_p,
                    );
                    write_g2(
                        &mut buffer[PAIR_ELEMENT_LEN * params.input_size - G2_LEN..],
                        &q,
                    );

                    let result = run_pair(
                        &buffer[..params.input_size * PAIR_ELEMENT_LEN],
                        PAIR_PER_POINT_COST,
                        PAIR_BASE_COST,
                        PRECOMPILE_CALL_MAX_GAS,
                    )
                    .unwrap();
                    assert_eq!(result.bytes[31], 1); // success

                    buffer = &mut buffer[params.input_size * PAIR_ELEMENT_LEN..];
                }
            }),
            stack_builder: Box::new(move |stack, params| {
                let arg_size = params.input_size * PAIR_ELEMENT_LEN;
                for i in 0..params.repetition {
                    assert!(stack.push(U256::ZERO)); // retSize
                    assert!(stack.push(U256::ZERO)); // retOffset
                    assert!(stack.push(U256::from(arg_size))); // argsSize
                    assert!(stack.push(U256::from(i * arg_size))); // argsOffset
                    assert!(stack.push(U256::from_be_slice(addr.as_slice()))); // address
                    assert!(stack.push(U256::from(PRECOMPILE_CALL_MAX_GAS))); // gas
                }
            }),
            bytecode_builder: default_bytecode_with_pop_builder(OpCode::STATICCALL),
            ..Default::default()
        }),
    );
}

fn call_stack_builder(addr: Address, arg_size: usize, gas: u64) -> StackBuilder {
    Box::new(move |stack, params| {
        for i in 0..params.repetition {
            assert!(stack.push(U256::ZERO)); // retSize
            assert!(stack.push(U256::ZERO)); // retOffset
            assert!(stack.push(U256::from(arg_size))); // argsSize
            assert!(stack.push(U256::from(i * arg_size))); // argsOffset
            assert!(stack.push(U256::from_be_slice(addr.as_slice()))); // address
            assert!(stack.push(U256::from(gas))); // gas
        }
    })
}

mod bn128 {
    use ark_ec::AffineRepr;
    use ark_serialize::CanonicalSerialize;
    use rand::{Rng, RngCore};
    use std::ops::Mul;

    pub use ark_bn254::{Fr, G1Affine, G2Affine};

    pub use revm_precompile::bn128::{
        ADD_INPUT_LEN, MUL_INPUT_LEN, PAIR_ELEMENT_LEN,
        add::ISTANBUL_ADD_GAS_COST as ADD_GAS_COST,
        mul::ISTANBUL_MUL_GAS_COST as MUL_GAS_COST,
        pair::{
            ISTANBUL_PAIR_BASE as PAIR_BASE_COST, ISTANBUL_PAIR_PER_POINT as PAIR_PER_POINT_COST,
        },
        run_add, run_mul, run_pair,
    };

    /// FQ_LEN specifies the number of bytes needed to represent an
    /// Fq element. This is an element in the base field of BN254.
    ///
    /// Note: The base field is used to define G1 and G2 elements.
    pub const FQ_LEN: usize = 32;

    /// SCALAR_LEN specifies the number of bytes needed to represent an Fr element.
    /// This is an element in the scalar field of BN254.
    pub const SCALAR_LEN: usize = 32;

    /// FQ2_LEN specifies the number of bytes needed to represent an
    /// Fq^2 element.
    ///
    /// Note: This is the quadratic extension of Fq, and by definition
    /// means we need 2 Fq elements.
    pub const FQ2_LEN: usize = 2 * FQ_LEN;

    /// G1_LEN specifies the number of bytes needed to represent a G1 element.
    ///
    /// Note: A G1 element contains 2 Fq elements.
    pub const G1_LEN: usize = 2 * FQ_LEN;
    /// G2_LEN specifies the number of bytes needed to represent a G2 element.
    ///
    /// Note: A G2 element contains 2 Fq^2 elements.
    pub const G2_LEN: usize = 2 * FQ2_LEN;

    pub fn write_g1<'a>(buffer: &'a mut [u8], point: &G1Affine) -> &'a mut [u8] {
        let mut serialize_le = [0u8; FQ_LEN];
        let (x, y) = point.xy().unwrap();

        x.serialize_uncompressed(&mut serialize_le[..]).unwrap();
        serialize_le.reverse();
        buffer[..FQ_LEN].copy_from_slice(&serialize_le);

        y.serialize_uncompressed(&mut serialize_le[..]).unwrap();
        serialize_le.reverse();
        buffer[FQ_LEN..2 * FQ_LEN].copy_from_slice(&serialize_le);

        &mut buffer[2 * FQ_LEN..]
    }

    pub fn write_g2<'a>(buffer: &'a mut [u8], point: &G2Affine) -> &'a mut [u8] {
        let mut serialize_le = [0u8; FQ2_LEN];
        let (x, y) = point.xy().unwrap();

        x.serialize_uncompressed(&mut serialize_le[..]).unwrap();
        serialize_le.reverse();
        buffer[..FQ2_LEN].copy_from_slice(&serialize_le);

        y.serialize_uncompressed(&mut serialize_le[..]).unwrap();
        serialize_le.reverse();
        buffer[FQ2_LEN..2 * FQ2_LEN].copy_from_slice(&serialize_le);

        &mut buffer[2 * FQ2_LEN..]
    }

    #[inline(always)]
    pub fn rand_g1<R: RngCore>(rng: R) -> G1Affine {
        let g = G1Affine::generator();
        let g1 = G1Affine::from(g.mul(rand_scalar(rng)));
        assert!(g1.is_on_curve());
        assert!(g1.is_in_correct_subgroup_assuming_on_curve());
        g1
    }

    #[inline(always)]
    pub fn rand_g2<R: RngCore>(rng: R) -> G2Affine {
        let g = G2Affine::generator();
        G2Affine::from(g.mul(rand_scalar(rng)))
    }

    #[inline(always)]
    pub fn rand_scalar<R: RngCore>(mut rng: R) -> Fr {
        Fr::from(rng.random::<u128>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_repetition() {
        let mut map = BTreeMap::new();
        fill(&mut map);

        for (_, builder) in map.iter() {
            for tc in builder.build_all(Some(42)) {
                let repetition = tc.repetition;
                let opcodes = tc.count_opcodes();
                assert_eq!(opcodes.get(OpCode::STATICCALL), Some(repetition));
            }
        }
    }
}
