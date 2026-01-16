#![no_main]

use libfuzzer_sys::fuzz_target;
use orzatty_core::frame::FrameHeader;

fuzz_target!(|data: &[u8]| {
    // Try to decode arbitrary bytes as a FrameHeader.
    // The function should NEVER panic, only return Err.
    let _ = FrameHeader::decode(data);
    
    // If we got a valid header, try encoding it back
    if let Ok((header, _bytes_read)) = FrameHeader::decode(data) {
        let mut buf = [0u8; 64];
        // Encoding should also never panic
        let _ = header.encode(&mut buf);
    }
});
