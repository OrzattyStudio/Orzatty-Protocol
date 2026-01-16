//! Protocol state machine and high-level data structures.

use rkyv::{Archive, Deserialize, Serialize};

/// Example of a Zero-Copy structure for gaming state updates.
/// 
/// `#[derive(Archive)]` allows this to be written directly to bytes
/// and read back without parsing.
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
#[archive(check_bytes)] // Enables validation for safe zero-copy
#[repr(C)] // Ensure C-compatible layout for maximum predictability
pub struct PlayerUpdate {
    pub id: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub velocity: [f32; 3],
    pub status: u8,
}

// Ensure the structure is aligned properly
// rkyv handles alignment, but explicit repr(C) is good practice for network protocols.

