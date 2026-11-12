#![no_main]
//! fuzz هدف ۱ — پارسر `MintRequest` — هر panic یک باگ مرزی است.
use libfuzzer_sys::fuzz_target;
use dropagentx_contracts::MintRequest;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<MintRequest>(data);
});
