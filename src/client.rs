use std::{io, net::UdpSocket, process::exit};

use crate::{
    comm::Actions,
    frame_buffer::{FrameBuffer, GetFrameResult},
    packet::Packet,
    shared::{Shared, shard}
};
use ggez::{
    event,
    glam::*,
    graphics::{self, DrawParam, Drawable},
    Context, GameResult,
};

struct MainState {
    texture: Option<graphics::Image>,
    frames: Shared<FrameBuffer>,
    socket: Shared<UdpSocket>,
    process_handle: tokio::task::JoinHandle<()>, // Handle to the socket process. Used to stop the process when the game is closed
}

impl MainState {
    fn new(socket: UdpSocket, ctx: &mut Context) -> GameResult<MainState> {
        ctx.gfx
            .set_resizable(true)
            .expect("Error setting window to resizable");
        
        ctx.gfx.set_window_title("Screen Stream Client");

        let shared_socket: Shared<UdpSocket> = shard!(socket);
        let shared_frames: Shared<FrameBuffer> = shard!(FrameBuffer::new());
        
        // UdpSocket proces
        let handle = tokio::spawn({
            let args = (shared_socket.clone(), shared_frames.clone() );            
            async move { handle_socket(args.0, args.1).await; }
        });
        
        Ok(MainState {
            texture: None,
            frames: shared_frames,
            socket: shared_socket,
            process_handle: handle,
        })
    }
}

/// Receive data from the server
async fn handle_socket(shared_socket: Shared<UdpSocket>, shared_frames: Shared<FrameBuffer>) {
    
    // * Frame will be sent in packets of CHUNK_SIZE
    let mut buffer = [0u8; Packet::CHUNK_SIZE * 1];

    loop {
        let socket =  shared_socket.consume();
        match socket.recv(&mut buffer) {
            Ok(bytes_read) => {
                // println!("Bytes read: {}", bytes_read);
    
                // No bytes read means server closed the connection
                if bytes_read == 0 {
                    println!("Server closed the connection");
                    exit(0);
                }
                // If not even minimum bytes are read
                else if bytes_read < Packet::META_SIZE {
                    eprintln!(
                        "Invalid packet received, Expected at least: {} bytes, recieved: {}",
                        Packet::META_SIZE,
                        bytes_read
                    );
                    continue;
                }
    
                if bytes_read <= Packet::CHUNK_SIZE {
                    let packet = Packet::from_bytes(buffer[..bytes_read].to_vec());
                    // println!(
                    //     "Single packet received frame: {} index: {}",
                    //     packet.frame_id, packet.index
                    // );
    
                    shared_frames.consume().add_packet(packet);
                }
            }
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::WouldBlock => {
                        // println!("No data available");
                    }
                    io::ErrorKind::ConnectionReset => {
                        println!("Connection reset by server");
                        exit(0);
                    }
                    _ => {
                        eprintln!("Error receiving data: {:?}", e);
                        exit(1);
                    }
                }
            }
        }
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn quit_event(&mut self, _ctx: &mut Context) -> Result<bool, ggez::GameError> {
        // Send disconnection notification
        self.socket.lock().unwrap().send(&[Actions::Disconnection as u8])
            .expect("Error sending disconnection notification to server");

        return Ok(false);
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // Check if stream is still open
        if self.socket.lock().unwrap().send(&[Actions::Ping as u8]).is_err() {
            println!("Stream is closed");
            self.process_handle.abort();
            exit(0);
        }

        // No frames -> return
        if self.frames.lock().unwrap().len() == 0 {
            return Ok(());
        }

        // println!("Frame buffer count: {}", self.frames.len());

        let buffer = match self.frames.lock().unwrap().get_frame() {
            GetFrameResult::NoFrame => {
                return Ok(());
            }

            GetFrameResult::NonSequential(packets) => {
                println!(
                    "Not sequential packet: {:?}",
                    packets.iter().map(|p| p.index).collect::<Vec<u8>>()
                );
                return Ok(());
            }

            GetFrameResult::Ok(buffer) => buffer,
        };

        // * Convert image to texture
        match graphics::Image::from_bytes(&ctx.gfx, &buffer) {
            Ok(texture) => {
                self.texture = Some(texture);
            }
            Err(e) => {
                eprintln!("Error converting image to texture: {:?}", e);
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);

        // Display the image
        if let Some(texture) = &self.texture {
            // texture needs to fit the screen
            let (w, h) = ctx.gfx.size();

            let dest_point = Vec2::new(0.0, 0.0);
            // texture.draw(&mut canvas, DrawParam::new().dest(dest_point));

            texture.draw(
                &mut canvas,
                DrawParam::new()
                    .dest(dest_point)
                    .transform(ggez::mint::ColumnMatrix4 {
                        x: Vec4::new(w as f32 / texture.width() as f32, 0.0, 0.0, 0.0).into(),
                        y: Vec4::new(0.0, h as f32 / texture.height() as f32, 0.0, 0.0).into(),
                        z: Vec4::new(0.0, 0.0, 1.0, 0.0).into(),
                        w: Vec4::new(0.0, 0.0, 0.0, 1.0).into(),
                    }),
            );
        }
        canvas.finish(ctx)?;
        Ok(())
    }
}

pub fn run(address: String) -> GameResult {
    let cb: ggez::ContextBuilder = ggez::ContextBuilder::new("ss-client", "nova");
    let (mut ctx, event_loop) = cb.build()?;

    let socket: UdpSocket =
        UdpSocket::bind(format!("0.0.0.0:{}", 8899)).expect("Error binding to address");

    socket
        .set_nonblocking(true)
        .expect("Error setting socket to non-blocking");

    socket
        .connect(&address)
        .expect("Error connecting to address");

    // 1 = Connection notification
    socket
        .send(&[Actions::NewConnection as u8])
        .expect("Error sending connection notification to server");

    println!("Connected to: {}", address);

    let state = MainState::new(socket, &mut ctx)?;

    event::run(ctx, event_loop, state);
}
