#![no_main]
//! fuzz هدف ۳ — پارسر `AiChatRequest` (قراردادهای AI — حداقل ADR-009).
use libfuzzer_sys::fuzz_target;
use dropagentx_contracts::AiChatRequest;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<AiChatRequest>(data);
});
