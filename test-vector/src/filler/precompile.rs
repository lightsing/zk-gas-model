use crate::{TestCaseBuilder, TestCaseKind, filler::default_bytecode_with_pop_builder};
use ark_bn254::{Fq, Fr, G1Affine};
use ark_ec::AffineRepr;
use ark_ff::{BigInt, Field, One, ToConstraintField};
use ark_serialize::CanonicalSerialize;
use rand::Rng;
use revm_bytecode::OpCode;
use revm_precompile::u64_to_address;
use revm_primitives::{Address, U256};
use std::{collections::BTreeMap, ops::Sub, sync::Arc};

pub(crate) fn fill(map: &mut BTreeMap<Address, Arc<TestCaseBuilder>>) {
    fill_ec_mul(map);
}

fn fill_ec_mul(map: &mut BTreeMap<Address, Arc<TestCaseBuilder>>) {
    // G1Affine
    let addr = u64_to_address(0x07);
    map.insert(
        addr,
        Arc::new(TestCaseBuilder {
            description: Arc::from("ECMUL"),
            kind: TestCaseKind::DynamicMixed,
            support_repetition: 1..1024 / OpCode::STATICCALL.inputs() as usize,
            support_input_size: (0..254).collect(), // how many 1 in the scalar bits
            memory_builder: Box::new(|memory, params| {
                let mut rng = params.rng();

                let memory_size = params.repetition * 32 * 3;
                memory.resize(memory_size);
                let mut context_memory_mut = memory.context_memory_mut();
                let mut buffer = context_memory_mut.as_mut();

                // scalar = 2^input_size - 1
                // we won't exceed the 254 bits of Fr
                let scalar = U256::from(2u8)
                    .pow(U256::from(params.input_size as u32))
                    .sub(U256::ONE)
                    .to_be_bytes::<32>();

                let g = G1Affine::generator();
                for _ in 0..params.repetition {
                    let point = G1Affine::from(g.mul_bigint(rng.random::<U256>().as_limbs()));
                    let (x, y) = point.xy().unwrap();
                    x.serialize_uncompressed(&mut buffer[..32]).unwrap();
                    y.serialize_uncompressed(&mut buffer[32..64]).unwrap();
                    buffer[64..96].copy_from_slice(&scalar);
                    buffer = &mut buffer[96..];
                }
            }),
            stack_builder: Box::new(move |stack, params| {
                for i in 0..params.repetition {
                    assert!(stack.push(U256::ZERO)); // retSize
                    assert!(stack.push(U256::ZERO)); // retOffset
                    assert!(stack.push(U256::from(96))); // argsSize
                    assert!(stack.push(U256::from(i * 96))); // argsOffset
                    assert!(stack.push(U256::from_be_slice(addr.as_slice()))); // address
                    assert!(stack.push(U256::from(6000))); // gas
                }
            }),
            bytecode_builder: default_bytecode_with_pop_builder(OpCode::STATICCALL),
            ..Default::default()
        }),
    );
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::filler::TestCaseBuilder;
//     use evm_guest::{Context, EthInterpreter, InstructionTable};
//     use revm_context::{Evm, result::EVMError};
//     use revm_handler::{
//         EthFrame, EthPrecompiles, EvmTr, Frame, FrameData, Handler, ItemOrResult, MainnetHandler,
//         PrecompileProvider, instructions::EthInstructions,
//     };
//     use revm_inspector::{Inspector, inspectors::GasInspector};
//     use revm_interpreter::{
//         CallInput, CallInputs, CallScheme, CallValue, FrameInput, InitialAndFloorGas, Interpreter,
//         InterpreterAction, InterpreterTypes, instruction_table,
//     };
//     use std::convert::Infallible;
// 
//     static INSTRUCTION_TABLE: InstructionTable = instruction_table();
// 
//     struct DebugInspector;
// 
//     impl Inspector<Context, EthInterpreter> for DebugInspector {
//         fn step(&mut self, interp: &mut Interpreter<EthInterpreter>, context: &mut Context) {
//             println!("1");
//         }
//     }
// 
//     // #[test]
//     // fn test_fill_ec_mul() {
//     //     let mut map = BTreeMap::new();
//     //     fill_ec_mul(&mut map);
//     //     let builder = map.get(&u64_to_address(0x07)).unwrap();
//     //     let tcs = builder.build_all(Some(42));
//     //
//     //     for tc in tcs.into_iter().skip(1).take(1) {
//     //         let mut interpreter = tc.interpreter;
//     //         println!("{:?}", interpreter.bytecode.bytecode());
//     //         println!("{:?}", interpreter.memory);
//     //         let context = tc.context_builder.build(tc.spec_id);
//     //         let precompiles = EthPrecompiles::default();
//     //         let mut evm = Evm::new(
//     //             context,
//     //             EthInstructions::<EthInterpreter, _>::new_mainnet(),
//     //             precompiles,
//     //         );
//     //
//     //         let mut handler = MainnetHandler::<
//     //             Evm<
//     //                 Context,
//     //                 DebugInspector,
//     //                 EthInstructions<EthInterpreter, Context>,
//     //                 EthPrecompiles,
//     //             >,
//     //             EVMError<Infallible>,
//     //             EthFrame<_, _, _>,
//     //         >::default();
//     //
//     //         let first_frame = FrameInput::Call(Box::new(CallInputs {
//     //             input: CallInput::default(),
//     //             gas_limit: u64::MAX,
//     //             target_address: builder.target_address,
//     //             bytecode_address: builder.target_address,
//     //             caller: builder.caller_address,
//     //             value: CallValue::Transfer(U256::ZERO),
//     //             scheme: CallScheme::Call,
//     //             is_static: false,
//     //             is_eof: false,
//     //             return_memory_offset: 0..0,
//     //         }));
//     //
//     //         // let InterpreterAction::NewFrame(input) = evm.run_interpreter(&mut interpreter) else {
//     //         //     return;
//     //         // };
//     //         // println!("input: {input:?}");
//     //         let first_frame_input = handler.first_frame_input(&mut evm, u64::MAX).unwrap();
//     //         let first_frame = handler
//     //             .first_frame_init(&mut evm, first_frame_input)
//     //             .unwrap();
//     //
//     //         let mut frame_result = match first_frame {
//     //             ItemOrResult::Item(mut frame) => {
//     //                 println!("first_frame: input {:?}", frame.input);
//     //                 frame.interpreter = interpreter;
//     //                 handler.run_exec_loop(&mut evm, frame).unwrap()
//     //             }
//     //             ItemOrResult::Result(result) => {
//     //                 println!("first_frame: result {:?}", result);
//     //                 result
//     //             }
//     //         };
//     //
//     //         println!("{:?}", frame_result.gas());
//     //         println!(
//     //             "before last_frame_result gas={:?}",
//     //             frame_result.gas().spent()
//     //         );
//     //         handler
//     //             .last_frame_result(&mut evm, &mut frame_result)
//     //             .unwrap();
//     //         println!(
//     //             "after last_frame_result gas={:?}",
//     //             frame_result.gas().spent()
//     //         );
//     //     }
//     // }
// }
