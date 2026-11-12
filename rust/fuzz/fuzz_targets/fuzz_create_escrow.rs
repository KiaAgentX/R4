#![no_main]
//! fuzz هدف ۲ — پارسر `CreateEscrowRequest`.
use libfuzzer_sys::fuzz_target;
use dropagentx_contracts::CreateEscrowRequest;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<CreateEscrowRequest>(data);
});
