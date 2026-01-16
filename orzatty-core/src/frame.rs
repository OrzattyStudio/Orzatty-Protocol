use bitflags::bitflags;
use crate::error::Error;

/// The type of data contained in the frame payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// Raw binary data, passed through as-is.
    RawBinary = 0x00,
    /// Data serialized with Rkyv (Archive).
    RkyvAligned = 0x01,
    /// UTF-8 encoded text.
    Utf8Text = 0x02,
    /// Reserved for future extensions or custom commands.
    Unknown = 0xFF,
}

impl From<u8> for FrameType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => FrameType::RawBinary,
            0x01 => FrameType::RkyvAligned,
            0x02 => FrameType::Utf8Text,
            _ => FrameType::Unknown,
        }
    }
}

bitflags! {
    /// Header flags for controlling frame processing.
    ///
    /// Layout (3 bits used):
    /// | 7 | 6 | 5 | 4 | 3 |  2  |  1  |  0  |
    /// | - | - | - | - | - | Res | Pri | Ctl |
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FrameFlags: u8 {
        /// Control Message (Ping, Pong, Close). 
        /// If set, this is NOT application data.
        const CONTROL = 0b1000_0000; // Bit 7
        
        /// High Priority.
        /// Should be processed immediately, bypassing normal queues if possible.
        const PRIORITY = 0b0100_0000; // Bit 6
        
        // Bit 5 is reserved for future use
    }
}

/// The wire-format header for an Orzatty Frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    pub flags: FrameFlags,
    pub frame_type: FrameType,
    pub channel_id: u32, // Application-level multiplexing
    pub stream_id: u64,
    pub length: u64,
}

impl FrameHeader {
    /// Encodes the header into a byte buffer.
    /// Returns the number of bytes written or an error if buffer is too small.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let mut offset = 0;
        
        if buf.len() < 1 {
            return Err(Error::BufferTooSmall { needed: 1, available: buf.len() });
        }

        let type_bits = (self.frame_type as u8) & 0x1F;
        let mut first_byte = type_bits;
        
        if self.flags.contains(FrameFlags::CONTROL) { first_byte |= 0b1000_0000; }
        if self.flags.contains(FrameFlags::PRIORITY) { first_byte |= 0b0100_0000; }
        
        buf[offset] = first_byte;
        offset += 1;

        offset += encode_varint(self.channel_id as u64, &mut buf[offset..])?;
        offset += encode_varint(self.stream_id, &mut buf[offset..])?;
        offset += encode_varint(self.length, &mut buf[offset..])?;
        
        Ok(offset)
    }

    /// Decodes the header from a byte buffer.
    pub fn decode(buf: &[u8]) -> Result<(Self, usize), Error> {
        if buf.is_empty() { 
            return Err(Error::IncompleteInput { needed_min: 1, available: 0 }); 
        }
        
        let first_byte = buf[0];
        let mut flags = FrameFlags::empty();
        if first_byte & 0b1000_0000 != 0 { flags |= FrameFlags::CONTROL; }
        if first_byte & 0b0100_0000 != 0 { flags |= FrameFlags::PRIORITY; }
        
        let frame_type = FrameType::from(first_byte & 0x1F);
        
        let mut offset = 1;
        
        let (channel_id_raw, len_c) = decode_varint(&buf[offset..])?;
        offset += len_c;
        let channel_id = channel_id_raw as u32;

        let (stream_id, len_s) = decode_varint(&buf[offset..])?;
        offset += len_s;
        
        let (length, len_l) = decode_varint(&buf[offset..])?;
        offset += len_l;
        
        Ok((FrameHeader {
            flags,
            frame_type,
            channel_id,
            stream_id,
            length,
        }, offset))
    }
}

// Minimal VarInt implementation (QUIC-style: 2 bits length, 6/14/30/62 bits value)
fn encode_varint(v: u64, buf: &mut [u8]) -> Result<usize, Error> {
    if v <= 63 {
        if buf.len() < 1 { return Err(Error::BufferTooSmall { needed: 1, available: 0 }); }
        buf[0] = v as u8;
        Ok(1)
    } else if v <= 16383 {
        if buf.len() < 2 { return Err(Error::BufferTooSmall { needed: 2, available: buf.len() }); }
        let bytes = ((v as u16) | 0x4000).to_be_bytes();
        buf[0..2].copy_from_slice(&bytes);
        Ok(2)
    } else if v <= 1073741823 {
        if buf.len() < 4 { return Err(Error::BufferTooSmall { needed: 4, available: buf.len() }); }
        let bytes = ((v as u32) | 0x80000000).to_be_bytes();
        buf[0..4].copy_from_slice(&bytes);
        Ok(4)
    } else {
        if buf.len() < 8 { return Err(Error::BufferTooSmall { needed: 8, available: buf.len() }); }
        let bytes = (v | 0xC000000000000000).to_be_bytes();
        buf[0..8].copy_from_slice(&bytes);
        Ok(8)
    }
}

fn decode_varint(buf: &[u8]) -> Result<(u64, usize), Error> {
    if buf.is_empty() { return Err(Error::IncompleteInput { needed_min: 1, available: 0 }); }
    let first = buf[0];
    let prefix = first >> 6;
    let length = 1 << prefix; // 00->1, 01->2, 10->4, 11->8
    
    if buf.len() < length { 
        return Err(Error::IncompleteInput { needed_min: length, available: buf.len() }); 
    }
    
    let res = match length {
        1 => (first & 0x3F) as u64,
        2 => u16::from_be_bytes([buf[0] & 0x3F, buf[1]]) as u64,
        4 => u32::from_be_bytes([buf[0] & 0x3F, buf[1], buf[2], buf[3]]) as u64,
        8 => u64::from_be_bytes([buf[0] & 0x3F, buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7]]),
        _ => return Err(Error::InvalidVarInt),
    };
    
    Ok((res, length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header_encode_decode() {
        let header = FrameHeader {
            flags: FrameFlags::CONTROL | FrameFlags::PRIORITY,
            frame_type: FrameType::Utf8Text, // 0x02
            channel_id: 42,
            stream_id: 12345,
            length: 100,
        };

        let mut buf = [0u8; 32];
        let written = header.encode(&mut buf).unwrap();
        
        // Flags: 11000000 (0xC0), Type: 00000010 (0x02) -> 11000010 (0xC2)
        assert_eq!(buf[0], 0xC2); 
        
        let (decoded, read_bytes) = FrameHeader::decode(&buf[..written]).unwrap();
        
        assert_eq!(decoded.flags, header.flags);
        assert_eq!(decoded.frame_type, header.frame_type);
        assert_eq!(decoded.channel_id, header.channel_id);
        assert_eq!(decoded.stream_id, header.stream_id);
        assert_eq!(decoded.length, header.length);
        assert_eq!(written, read_bytes);
    }
    
    #[test]
    fn test_varint() {
        let mut buf = [0u8; 8];
        
        let val = 15293;
        let n = encode_varint(val, &mut buf).unwrap();
        let (decoded, read) = decode_varint(&buf[..n]).unwrap();
        assert_eq!(val, decoded);
        assert_eq!(n, read);
    }

    #[test]
    fn test_errors() {
        let mut buf = [0u8; 1];
        // 15293 needs 2 bytes
        match encode_varint(15293, &mut buf) {
            Err(Error::BufferTooSmall { .. }) => {}, // Pass
            _ => panic!("Should have failed with BufferTooSmall"),
        }
    }
}
