use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use turbojpeg::{compress, Image, PixelFormat};

use crate::comm::Actions;
use crate::commands;
use crate::packet::Packet;
use bytes::Bytes;

use captrs::{Bgr8, CaptureError, Capturer};

use crate::shared::{Shared, shard};

pub fn run(options: commands::StartCmd) {
    let mut cap = Capturer::new(0).expect("Failed to create capturer");

    let (width, height) = cap.geometry();

    let shared_listener: Shared<UdpSocket> = shard!(
        UdpSocket::bind(format!("0.0.0.0:{}", options.port))
        .expect("While creating UdpSocket: Error binding to port")
    );

    let mut shared_clients: Vec<Shared<SocketAddr>> = Vec::new();
    let mut shared_to_remove: Shared<Vec<SocketAddr>> = shard!(Vec::new());

    shared_listener
        .lock()
        .unwrap()
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
            "\rStreaming since: {:#}\t",
            record_start.elapsed().as_secs_f64()
        );

        // * Handle incoming connections and disconnections
        let mut buffer = [0u8; 1];

        match shared_listener.lock().unwrap().recv_from(&mut buffer) {
            Ok((_amount, address)) => {
                match Actions::from(buffer[0]) {
                    // Ping
                    Actions::Ping => {}

                    // New connection
                    Actions::NewConnection => {
                        println!("Client Connected");
                        shared_clients.push(Arc::new(Mutex::new(address)));
                    }

                    // Disconnection
                    Actions::Disconnection => {
                        println!("Client Disconnected");
                        shared_clients.retain(|&x| *x.lock().unwrap() != address);
                    }

                    Actions::Unknown => {
                        println!("Received Unknown Message: {} from {}", buffer[0], address);
                    }
                }
            }
            Err(_e) => {}
        }

        if shared_clients.len() == 0 {
            // println!("No clients connected");
            std::thread::sleep(fps);
            continue;
        }

        // * Sending frames to clients
        let frame: Vec<Bgr8> = match cap.capture_frame() {
            Ok(frame) => frame,
            Err(err) => {
                match err {
                    // Skip these errors
                    CaptureError::AccessDenied
                    | CaptureError::Timeout
                    | CaptureError::RefreshFailure
                    | CaptureError::AccessLost => {
                        continue;
                    }

                    // Serious errors
                    CaptureError::Fail(e) => {
                        eprintln!("Error capturing frame: {}", e);
                        continue;
                    }
                }
            }
        };

        // Covert frame: Vec of BGR8 to &[u8]. A &[u8] is required for turbojpeg to create an image...
        // BGR8 is just a struct with 3 u8 values
        let bytes = frame
            .iter()
            .flat_map(|x| vec![x.b, x.g, x.r])
            .collect::<Vec<u8>>();

        let image = Image {
            pixels: bytes.as_slice(),
            width: width as usize,
            height: height as usize,
            format: PixelFormat::BGR,
            pitch: (width as usize) * PixelFormat::BGR.size(),
        };

        println!("Frame Size: {}", frame.len());

        // Bytes that will be sent to the clients
        let img_bytes = compress(image, options.quality as i32, turbojpeg::Subsamp::Sub2x2)
            .expect("Error compressing image");

        println!("Compressed Frame Size: {}", bytes.len());
        
        let bytes = Bytes::from(img_bytes.to_vec());
        
        // Frame ID - unique identifier for the frame
        let frame_id = record_start.elapsed().as_millis() as u32;

        // * Send frame to all connected clients
        for client in &shared_clients {
            tokio::spawn(async move {
                send_frame(shared_listener.clone(), client.clone(), bytes.clone(), shared_to_remove.clone(), frame_id).await;
            });
        }

        let to_remove = *match shared_to_remove.lock() {
            Ok(to_remove) => to_remove,
            Err(e) => {
                eprintln!("Error locking to_remove: {:?}", e);
                continue;
            }
        };
        
        if to_remove.len() == shared_clients.len() {
            println!("All clients disconnected");
            break;
        }

        // * Remove clients with errors
        if to_remove.len() > 0 {
            for client in &to_remove {
                shared_clients.retain(|&x| *x.lock().unwrap() != *client);
            }
            to_remove.clear();
        }

        // Stop at 1 minute
        if record_start.elapsed().as_secs() > 60 {
            break;
        }
    }
}


async fn send_frame(
    shared_socket: Shared<UdpSocket>,
    client: Shared<SocketAddr>, 
    bytes: Bytes,
    clients_to_remove: Shared<Vec<SocketAddr>>,
    frame_id: u32,
) {
    let listener = match shared_socket.lock() {
        Ok(listener) => listener,
        Err(e) => {
            println!("Error locking UdpSocket: {}", e);
            return;
        }
    };
    
    let client = *client.lock().unwrap();
    
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
                clients_to_remove.lock().unwrap().push(client);
                break;
            }
        }
    }
}