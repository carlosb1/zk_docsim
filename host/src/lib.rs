use risc0_zkvm::{default_prover, ExecutorEnv, ProveInfo};
use methods::GUEST_CODE_FOR_ZK_PROOF_ELF;

pub fn execute_prove(embedding1: Vec<f32>, embedding2: Vec<f32>) -> ProveInfo {
    let env = ExecutorEnv::builder().write(&(embedding1, embedding2)).unwrap().build().unwrap();

    // Obtain the default prover.
    let prover = default_prover();

    let prove_info = prover
        .prove(env, GUEST_CODE_FOR_ZK_PROOF_ELF)
        .unwrap();
    prove_info
}

pub fn execute_and_serialize_receipt(embedding1: Vec<f32>, embedding2: Vec<f32>) -> anyhow::Result<Vec<u8>> {
    let prove_info = execute_prove(embedding1, embedding2);
    let receipt = prove_info.receipt;
    Ok(bincode::serialize(&receipt)?)
}