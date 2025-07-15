use std::fs;
use wasm_bindgen::prelude::*;
use risc0_zkvm::serde::from_slice;
use risc0_zkvm::Receipt;

//TODO hardcoded zk proof
pub const GUEST_CODE_FOR_ZK_PROOF_ID: [u32; 8] = [4239518147, 485871892, 529799477, 3277490305, 2515661234, 362053723, 2729086396, 2562830873];
#[wasm_bindgen]
pub fn verify_receipt(receipt_bytes: &[u8], image_id: &[u32]) -> Result<(), JsValue> {
    let receipt: Receipt = bincode::deserialize(receipt_bytes)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e}")))?;

    receipt
        .verify(GUEST_CODE_FOR_ZK_PROOF_ID)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {e}")))
}