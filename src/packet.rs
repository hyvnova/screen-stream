use std::cmp::Ordering;

// UDP packet
pub struct Packet {
    pub index: u8,      // First byte Index of the packet
    pub frame_id: u32,  // Frame ID
    pub data: Vec<u8>, // Data of the packet
}

impl Packet {
    pub const META_SIZE : usize = 5;
    
    // Limit 65507
    pub const CHUNK_SIZE : usize = 65000;

    pub fn new(index: u8, frame_id: u32, data: &[u8]) -> Self {
        Self { index, frame_id, data: data.to_vec() }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.index.to_le_bytes()[0]];
        bytes.extend_from_slice(&self.frame_id.to_le_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            index: bytes[0],
            frame_id: u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
            data: bytes[5..].to_vec(),
        }
    }
}


impl PartialOrd for Packet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Different frame_id -> Not comparable
        if self.frame_id != other.frame_id {
            return None;
        }

        // Compare index
        self.index.partial_cmp(&other.index)
    }
}

impl PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {    
        self.index == other.index && self.frame_id == other.frame_id
    }
}

impl Clone for Packet {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            frame_id: self.frame_id,
            data: self.data.clone(),
        }
    }
}