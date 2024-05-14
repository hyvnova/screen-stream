use std::collections::HashMap;

use crate::packet::Packet;

/// Data structure to store frame packets
/// Ensures that only 3 frames are stored at a time
pub struct FrameBuffer {
    pub frames : HashMap<u32, Vec<Packet>>,
    order: Vec<u32> // Order of frames
}


/// Possible results when getting a frame from the frame buffer
/// NoFrame - No frame is available
/// NonSequential - Frame is not complete or packets are sequential
/// Ok(Vec<u8>) - Frame is complete and the data is returned as a Vec<u8>
pub enum GetFrameResult {
    NoFrame,
    NonSequential(Vec<Packet>),
    Ok(Vec<u8>)
}

impl FrameBuffer {
    const MAX_FRAMES: usize = 3;

    pub fn new() -> Self {
        Self {
            frames: HashMap::new(),
            order: Vec::new()
        }
    }

    /// Adds a packet to the frame buffer
    /// Ensures no-duplicate packets are added
    /// Ensures packets are added in order
    /// This function should be called after adding the frame
    fn add_packet_to_frame(&mut self, packet: Packet) {
        let frame = self.frames.get_mut(&packet.frame_id).unwrap(); 
        if !frame.contains(&packet) {
            // Find the index to insert the packet
            let index = frame.iter().position(|p| p.index > packet.index).unwrap_or(frame.len());
            frame.insert(index, packet);
        }
    }

    /// Creates a new frame
    /// If the frame is already present, it will be overwritten
    /// If the frame buffer has more than 3 frames, the oldest frame will be removed
    fn create_frame(&mut self, frame_id: u32) {
        if self.frames.len() >= Self::MAX_FRAMES {
            let oldest_frame = self.order.remove(0);
            self.frames.remove(&oldest_frame);
        }

        self.frames.insert(frame_id, Vec::new());
        self.order.push(frame_id);
    }

    /// Add a packet to the frame buffer
    /// If the frame is not present, create a new frame
    pub fn add_packet(&mut self, packet: Packet) {

        // Create new frame if not present
        if !self.frames.contains_key(&packet.frame_id) {
            self.create_frame(packet.frame_id);
        }
        // add packet to the frame
        self.add_packet_to_frame(packet);
    }    



    /// Get the oldest frame
    /// If frame buffer is not *complete* next oldest frame will be returned
    /// A complete frame is that whose last packet data size is lass than Packet::CHUNK_SIZE
    /// If no frame is complete, None will be returned
    pub fn get_frame(&mut self) -> GetFrameResult {
        if self.frames.len() == 0 {
            return GetFrameResult::NoFrame;
        }

        let mut frame_id = 0;

        loop {
            let frame = match self.frames.get(&self.order[frame_id]) {
                Some(frame) => frame,
                None => return GetFrameResult::NoFrame
            };

            // Check if frame is complete -- has a last packet
            if frame.last().unwrap().data.len() < Packet::CHUNK_SIZE {
                break;
            }

            frame_id += 1;
            if frame_id >= Self::MAX_FRAMES {
                return GetFrameResult::NoFrame;
            }
        }

        let packets = self.frames.get(&self.order[frame_id]).unwrap();

        // Check if packets are sequential
        if packets
            .iter()
            .enumerate()
            .any(|(i, packet)| packet.index as usize != i)
        {
            return GetFrameResult::NonSequential(packets.to_vec());
        }


        // Create frame buffer
        let buffer_size = packets
            .iter()
            .fold(0, |acc, packet| acc + packet.data.len());
        
        let mut buffer: Vec<u8> = Vec::with_capacity(buffer_size.into());

        for packet in packets {
            println!(
                "Building frame: {} index: {}",
                packet.frame_id, packet.index
            );
            buffer.extend_from_slice(&packet.data);
        }

        // Remove frame from the buffer
        self.frames.remove(&self.order[frame_id]);

        return GetFrameResult::Ok(buffer);
    }


    /// Returns the number of frames in the buffer
    pub fn len(&self) -> usize {
        self.frames.len()
    }
}


