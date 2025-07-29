use std::fs;
use wasm_bindgen::prelude::*;
use risc0_zkvm::serde::from_slice;
use risc0_zkvm::Receipt;

//TODO hardcoded zk proof
pub const GUEST_CODE_FOR_ZK_PROOF_ELF: &[u8] = include_bytes!("/home/carlosb/rust-workspace/zk_docsim/target/riscv-guest/methods/guest_code_for_zk_proof/riscv32im-risc0-zkvm-elf/release/guest_code_for_zk_proof.bin");
pub const GUEST_CODE_FOR_ZK_PROOF_PATH: &str = "/home/carlosb/rust-workspace/zk_docsim/target/riscv-guest/methods/guest_code_for_zk_proof/riscv32im-risc0-zkvm-elf/release/guest_code_for_zk_proof.bin";
pub const GUEST_CODE_FOR_ZK_PROOF_ID: [u32; 8] = [478837532, 155373028, 3133057801, 3741298655, 2812385851, 1482427873, 3341837043, 1574732756];

#[wasm_bindgen]
pub fn verify_receipt(receipt_bytes: &[u8], image_id: &[u32]) -> Result<(), JsValue> {
    let receipt: Receipt = bincode::deserialize(receipt_bytes)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e}")))?;

    receipt
        .verify(GUEST_CODE_FOR_ZK_PROOF_ID)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {e}")))
}