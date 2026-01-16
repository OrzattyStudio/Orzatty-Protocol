use rkyv::{Archive, Deserialize, Serialize};
extern crate alloc;
use alloc::string::String;

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
#[archive(check_bytes)]
#[repr(C)]
pub enum AuthMessage {
    /// Client sends this to authenticate.
    Hello { 
        token: String, 
    },
    /// Server responds with this if authentication succeeds.
    Ok,
    /// Server responds with this if authentication fails.
    Fail { 
        reason: String, 
    },
}
