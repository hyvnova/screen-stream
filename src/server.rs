use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use bytes::Bytes;
use captrs::{Bgr8, CaptureError, Capturer};
use turbojpeg::{compress, Image, PixelFormat};

use crate::comm::Actions;
use crate::commands;
use crate::packet::Packet;
use crate::shared::{shard, Shared};

type ProcessedFrame = (Bytes, u32); // Frame data and frame_id

pub async fn run(options: commands::StartCmd) {
    let mut cap = Capturer::new(0).expect("Failed to create capturer");

    let (width, height) = cap.geometry();
    let size = (
        width as usize,
        height as usize,
    );

    let shared_listener: Shared<UdpSocket> =
        shard!(UdpSocket::bind(format!("0.0.0.0:{}", options.port))
            .expect("While creating UdpSocket: Error binding to port"));

    let shared_clients: Shared<Vec<SocketAddr>> = shard!(Vec::new());
    let shared_processed_frames: Shared<Vec<ProcessedFrame>> = shard!(Vec::new());

    shared_listener
        .consume()
        .set_nonblocking(true)
        .expect("Error setting UdpSocket to non-blocking mode");

    println!("Server listening on port: {}", options.port);

    let fps = Duration::from_millis(1000u64 / (options.fps as u64)); // Frame time
    let record_start = std::time::Instant::now(); // Time since recording started

    println!("Frame Time: {:?}", fps);

    // ! Main loop
    loop {
        std::io::stdout().flush().unwrap();

        print!(
            "\rStreaming since: {:.2} seconds",
            record_start.elapsed().as_secs_f64()
        );

        // * Handle incoming connections and disconnections
        let mut buffer = [0u8; 1];

        match shared_listener.consume().recv_from(&mut buffer) {
            Ok((_amount, address)) => {
                match Actions::from(buffer[0]) {
                    // Ping
                    Actions::Ping => {}

                    // New connection
                    Actions::NewConnection => {
                        println!("Client Connected");
                        shared_clients.consume().push(address);
                    }

                    // Disconnection
                    Actions::Disconnection => {
                        println!("Client Disconnected");
                        shared_clients.consume().retain(|x| *x != address);
                    }

                    Actions::Unknown => {
                        println!("Received Unknown Message: {} from {}", buffer[0], address);
                    }
                }
            }
            Err(_e) => {}
        }

        if shared_clients.consume().len() == 0 {
            // println!("No clients connected");
            std::thread::sleep(fps);
            continue;
        }

        // * Process next frame
        // Capture frame -- Only if there are less than 5 frames in the buffer
        if shared_processed_frames.consume().len() == 0 {
            let frame: Vec<Bgr8> = match cap.capture_frame() {
                Ok(frame) => frame,
                Err(err) => {
                    match err {
                        CaptureError::Fail(reason) => {
                            println!("Failed to capture frame: {}", reason);
                        },

                        _ => {}
                    }
                    return;
                }
            };
            tokio::spawn({
                let args = (
                    shared_processed_frames.clone(),
                    size.clone(),
                );
                async move {
                    process_next_frame(
                        args.0, // Processed frames
                        args.1, // Size,
                        frame, // Frame
                        record_start.elapsed().as_millis() as u32, // Frame ID
                        options.quality
                    ).await;
                }
            });
        }

        // * Send processed frame to clients
        while shared_processed_frames.consume().len() > 0 {
            let (bytes, frame_id) = shared_processed_frames.consume().pop().unwrap();
            tokio::spawn({
                let args = (shared_listener.clone(), shared_clients.clone());
                async move {
                    send_frame(args.0, args.1, bytes, frame_id).await;
                }
            });
        }

        // Stop at 1 minute
        if record_start.elapsed().as_secs() > 60 {
            break;
        }
    }
}

async fn process_next_frame(
    shared_processed_frames: Shared<Vec<ProcessedFrame>>,
    size: (usize, usize),
    frame: Vec<Bgr8>,
    frame_id: u32,
    quality: u8,
) {
    // * Process frame


    // Covert frame: Vec of BGR8 to &[u8]. A &[u8] is required for turbojpeg to create an image...
    // BGR8 is just a struct with 3 u8 values
    let bytes = frame
        .iter()
        .flat_map(|x| vec![x.b, x.g, x.r])
        .collect::<Vec<u8>>();

    let image = Image {
        pixels: bytes.as_slice(),
        width: size.0 as usize,
        height: size.1 as usize,
        format: PixelFormat::BGR,
        pitch: (size.0 as usize) * PixelFormat::BGR.size(),
    };

    println!("Frame Size: {}", frame.len());

    // Bytes that will be sent to the clients
    let img_bytes = compress(image, quality as i32, turbojpeg::Subsamp::Sub2x2)
        .expect("Error compressing image");

    println!("Compressed Frame Size: {}", bytes.len());

    let bytes = Bytes::from(img_bytes.to_vec());

    shared_processed_frames.consume().push((bytes, frame_id));
}

async fn send_frame(
    shared_listener: Shared<UdpSocket>,
    shared_clients: Shared<Vec<SocketAddr>>,
    bytes: Bytes,
    frame_id: u32,
) {
    // * Send frame to all connected clients
    let listener = shared_listener.consume();
    let clients = shared_clients.consume();

    let mut to_remove: Vec<SocketAddr> = Vec::new();

    // * Frames are send on packets chunk size
    // Iterates over chunks and then iterates over clients to send the chunks :D
    for (i, chunk) in bytes
        .chunks(Packet::CHUNK_SIZE - Packet::META_SIZE)
        .enumerate()
    {
        let packet = Packet {
            index: i as u8,
            frame_id,
            data: chunk.to_vec(),
        };

        // I really don't want to nest this loop, but I don't know how to do it better <- Copilot wrote this
        for client in &*clients {
            match listener.send_to(&packet.to_bytes(), client) {
                Ok(bytes_send) => {
                    println!("\nPacket {} : size {}", i, bytes_send);
                }
                Err(e) => {
                    println!("Error sending packet to client: {}", e);
                    to_remove.push(*client);
                    break;
                }
            }
        }
    }

    if to_remove.len() == clients.len() {
        println!("All clients disconnected");
        return;
    }

    // * Remove clients with errors
    if to_remove.len() > 0 {
        for client in &to_remove {
            shared_clients.consume().retain(|x| *x != *client);
        }
    }
}
