//! Frame reader for QUIC streams.
//! 
//! This module handles reading Orzatty frames from QUIC streams,
//! managing buffering for fragmentation and coalescing.

use crate::frame::FrameHeader;
use crate::error::Error;
use bytes::{BytesMut, Buf};
use quinn::RecvStream;
use anyhow::{Result, anyhow};

/// Handles reading frames from a QUIC stream, managing buffering 
/// for fragmentation and coalescing.
pub struct Framer {
    buffer: BytesMut,
}

impl Framer {
    /// Create a new Framer with default buffer capacity.
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
        }
    }

    /// Create a new Framer with custom initial buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(capacity),
        }
    }

    /// Reads from the stream and tries to return the next complete frame payload.
    /// 
    /// Returns:
    /// - `Ok(Some((Header, BytesMut)))`: A complete frame.
    /// - `Ok(None)`: Stream finished cleanly.
    /// - `Err`: IO error or protocol violation.
    pub async fn read_frame(&mut self, stream: &mut RecvStream) -> Result<Option<(FrameHeader, BytesMut)>> {
        loop {
            // 1. Try to parse a frame from the current buffer
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // 2. If no complete frame, read more data from the network
            // We reserve space to avoid frequent allocations
            if self.buffer.capacity() < 1024 {
                self.buffer.reserve(4096);
            }

            // Read into a temporary buffer and extend BytesMut
            // Quinn's RecvStream::read returns Result<Option<usize>>
            let mut temp_buf = vec![0u8; 4096];
            match stream.read(&mut temp_buf).await? {
                Some(0) => {
                    // unexpected EOF if we have partial data
                    if !self.buffer.is_empty() {
                         return Err(anyhow!("Stream closed with partial frame data"));
                    }
                    return Ok(None);
                }
                Some(n) => {
                    // Extend the buffer with the read data
                    self.buffer.extend_from_slice(&temp_buf[..n]);
                    // Loop continues to try parsing again
                    continue;
                }
                None => {
                    if !self.buffer.is_empty() {
                        return Err(anyhow!("Stream closed with partial frame data"));
                    }
                    return Ok(None);
                }
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<(FrameHeader, BytesMut)>> {
        // We need at least 1 byte to start decoding header
        if self.buffer.is_empty() {
            return Ok(None);
        }

        match FrameHeader::decode(&self.buffer) {
            Ok((header, head_len)) => {
                let payload_len = header.length as usize;
                let total_len = head_len + payload_len;

                // Check if we have the full payload
                if self.buffer.len() >= total_len {
                    // Advance buffer past header
                    self.buffer.advance(head_len);
                    
                    // split_to returns the payload and advances state
                    let payload = self.buffer.split_to(payload_len);
                    
                    Ok(Some((header, payload)))
                } else {
                    // We have the header but not the full payload
                    Ok(None)
                }
            }
            Err(Error::IncompleteInput { .. }) => {
                // Not enough bytes for header
                Ok(None)
            }
            Err(Error::BufferTooSmall { .. }) => {
                 // Should not happen during decode, only encode
                 Err(anyhow!("Unexpected BufferTooSmall error during frame decode"))
            }
            Err(e) => {
                // Invalid data
                Err(anyhow!("Frame header decode error: {}", e))
            }
        }
    }

    /// Get the current buffer capacity (useful for debugging/monitoring).
    pub fn buffer_capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Get the current buffer length (bytes waiting to be parsed).
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl Default for Framer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;



    // Note: Full integration tests require actual QUIC streams
    // These are unit tests for the parsing logic
    #[test]
    fn test_framer_creation() {
        let framer = Framer::new();
        assert_eq!(framer.buffer_capacity(), 4096);
        assert_eq!(framer.buffer_len(), 0);

        let framer_custom = Framer::with_capacity(8192);
        assert_eq!(framer_custom.buffer_capacity(), 8192);
    }
}