use std::fs;
use wasm_bindgen::prelude::*;
use risc0_zkvm::serde::from_slice;
use risc0_zkvm::Receipt;

//HARDCODED ZK PROOF ID
pub const GUEST_CODE_FOR_ZK_PROOF_ID: [u32; 8] = [478837532, 155373028, 3133057801, 3741298655, 2812385851, 1482427873, 3341837043, 1574732756];


#[wasm_bindgen]
pub fn verify_receipt(receipt_bytes: &[u8]) -> Result<(), JsValue> {
    let receipt: Receipt = bincode::deserialize(receipt_bytes)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e}")))?;

    receipt
        .verify(GUEST_CODE_FOR_ZK_PROOF_ID)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {e}")))
}