use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use scrap::{Capturer, Display};
use turbojpeg::{Image, PixelFormat, compress};

use crate::comm::Actions;
use crate::commands;
use crate::packet::Packet;

use rgb::*;

use std::thread::available_parallelism;

pub fn run(options: commands::StartCmd) {
    let mut cap = Capturer::new(
        Display::primary()
        .expect("Failed to find primary display")
    ).expect("Failed to create capturer");


    let listener = UdpSocket::bind(format!("0.0.0.0:{}", options.port))
        .expect("While creating UdpSocket: Error binding to port");

    let mut clients = Vec::new(); // Connected clients

    listener
        .set_nonblocking(true)
        .expect("Error setting UdpSocket to non-blocking mode");

    println!("Server listening on port: {}", options.port);

    
    let width = cap.width();
    let height = cap.height();

    let fps = Duration::from_millis(1000u64 / (options.fps as u64)); // Frame time
    let record_start = std::time::Instant::now(); // Time since recording started

    // ! AVIF Encoder -- Very slow
    // let encoder = ravif::Encoder::new()
    //         .with_quality(options.quality as f32)
    //         .with_speed(10)
    //         .with_num_threads(match available_parallelism() {
    //             Ok(threads)  => Some(usize::from(threads)),
    //             Err(_) => None,
    //         });

    println!("Frame Time: {:?}", fps);

    // ! Main loop
    loop {

        println!("Streaming since: {:?}", record_start.elapsed());

        // * Handle incoming connections and disconnections
        let mut buffer = [0u8; 1];

        match listener.recv_from(&mut buffer) {
            Ok((_amount, address)) => {
                match Actions::from(buffer[0]) {
                    // Ping 
                    Actions::Ping => {
                        // listener.send_to(&[Actions::Ping as u8], address).expect("Failed to send pong");
                    }

                    // New connection
                    Actions::NewConnection => {
                        println!("Client Connected");
                        clients.push(address);
                    }

                    // Disconnection
                    Actions::Disconnection => {
                        println!("Client Disconnected");
                        clients.retain(|&x| x != address);
                    }

                    Actions::Unknown => {
                        println!("Received Unknown Message: {} from {}", buffer[0], address);
                    }
                }
            }
            Err(_e) => {
                // eprintln!("Error receiving from socket. Error: {:?}", _e);
            }
        }

        if clients.len() == 0 {
            println!("No clients connected");
            // wait whole frame time
            std::thread::sleep(fps);
            continue;
        }
        
        // * Sending frames to clients

        let start = std::time::Instant::now();
        
        // ! Frame Format
        // The frame format is guaranteed to be packed BGRA.
        // The width and height are guaranteed to remain constant.
        // The stride might be greater than the width, and it may also vary between frames.
        // Frame is just an array of bytes
        let frame = match cap.frame() {
            Ok(frame) => frame,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    // wait whole frame time
                     std::thread::sleep(fps);
                }
                continue;
            }
        };


        let image = Image {
            pixels: &*frame,
            width,
            height,
            format: PixelFormat::BGRX,
            pitch: width * PixelFormat::BGRX.size(),
        };

        println!("Frame Size: {}", frame.len());

        // * Encode & Compress frame as AVIF
        // let res = encoder.encode_rgba(
        //         ravif::Img::new(frame, width, height),
        //     )
        //     .expect("Error encoding frame");

        let bytes = compress(image, options.quality as i32, turbojpeg::Subsamp::Sub2x2).expect("Error compressing image");

        println!("Compressed Frame Size: {}", bytes.len());

        let mut clients_to_remove: Vec<SocketAddr> = Vec::new();

        // Frame ID - unique identifier for the frame
        let frame_id = record_start.elapsed().as_millis() as u32;

        // * Send frame to all connected clients
        for client in &clients {
    
            // * Frames are send on packets chunk size
            let chunk_size = Packet::CHUNK_SIZE;

            // chunk_size - 1 because the first byte is the index
            for (i, chunk) in bytes.chunks(chunk_size - Packet::META_SIZE).enumerate() {

                let packet = Packet {
                    index: i as u8,
                    frame_id,
                    data: chunk.to_vec(),
                };

                match listener.send_to(&packet.to_bytes(), client) {
                    Ok(bytes_send) => {
                        println!("\nPacket {} : size {}", i, bytes_send);
                    }
                    Err(e) => {
                        println!("Error sending packet to client: {}", e);
                        clients_to_remove.push(*client);
                        break;
                    }
                }
            }

        }

        if clients_to_remove.len() == clients.len() {
            println!("All clients disconnected");
            break;
        }

        // * Remove clients with errors
        for client in clients_to_remove {
            clients.retain(|&x| x != client);
        }

        // * Wait for the rest of the frame time
        let delta = start.elapsed();
        if delta < fps {
            std::thread::sleep(fps - delta);
        }
    }
} 