use std::{
    io::{self, Write},
    net::{SocketAddr, UdpSocket},
    process::exit,
    time::Instant,
};

use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    encoder::ImageEncoder,
    frame::{Frame, ImageFormat},
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};

use crate::{commands, packet::Packet};

// This struct will be used to handle the capture events.
pub struct Capture {
    // The video encoder that will be used to encode the frames.
    // encoder: Option<VideoEncoder>,
    // To measure the time the capture has been running
    pub start: Instant,
    listener: UdpSocket,
    clients: Vec<SocketAddr>,
    options: commands::StartCmd,
}

use turbojpeg;


impl GraphicsCaptureApiHandler for Capture {
    // The type of flags used to get the values from the settings.
    type Flags = String;

    // The type of error that can occur during capture, the error will be returned from `CaptureControl` and `start` functions.
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function that will be called to create the struct. The flags can be passed from settings.
    fn new(options_string: Self::Flags) -> Result<Self, Self::Error> {

        // let encoder = VideoEncoder::new(
        //     VideoEncoderType::Mp4,
        //     VideoEncoderQuality::HD720p,
        //     1920,
        //     1080,
        //     "./output.mp4",
        // )?;

        let options = commands::StartCmd::parse(options_string);

        let listener = UdpSocket::bind(format!("0.0.0.0:{}", options.port))
            .expect("While creating UdpSocket: Error binding to port");
        listener
            .set_nonblocking(true)
            .expect("Error setting UdpSocket to non-blocking mode");

        Ok(Self {
            listener,
            start: Instant::now(),
            clients: Vec::new(),
            options,
        })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // print!( "\rRecording for: {} seconds\n",  self.start.elapsed().as_secs() );
        io::stdout().flush()?;
        // ! ENCODER: Send the frame to the video encoder
        // self.encoder.as_mut().unwrap().send_frame(frame)?;

        // * Handle incoming connections.
        let mut buffer = [0u8; 1];

        match self.listener.recv_from(&mut buffer) {
            Ok((_amount, address)) => {
                match buffer[0] {
                    // No message
                    0 => {}

                    // New connection
                    1 => {
                        println!("Client Connected");
                        self.clients.push(address);
                    }

                    // Disconnection
                    2 => {
                        println!("Client Disconnected");
                        self.clients.retain(|&x| x != address);
                    }

                    _ => {
                        println!("Received Unknown Message: {} from {}", buffer[0], address);
                    }
                }
            }
            Err(_e) => {}
        }

        // println!("Connected Clients: {}", self.connected_count);

        // No client -> No need to send frame
        if self.clients.len() == 0 {
            return Ok(());
        }

        // * Prepare buffer for encoding
        let mut buffer = frame.buffer().expect("Error getting frame buffer");
        let width = buffer.width();
        let height = buffer.height();

        // * Encode frame as jpeg 
        let bytes = ImageEncoder::new(ImageFormat::Jpeg, ColorFormat::Rgba8)
            .encode(buffer.as_raw_nopadding_buffer()?, width, height)
            .expect("Error encoding frame into jpeg");

        // println!("Frame Size: {}", bytes.len());

        // * Compress frame
        let image = turbojpeg::decompress(&bytes, turbojpeg::PixelFormat::RGBA).expect("Error decompressing frame");

        let bytes = turbojpeg::compress(image.as_deref(), self.options.quality as i32, turbojpeg::Subsamp::Sub1x2).expect("Error compressing frame");

        // println!("Compressed Frame Size: {}", bytes.len());

        let mut clients_to_remove: Vec<SocketAddr> = Vec::new();

        // Frame ID - unique identifier for the frame
        let frame_id = self.start.elapsed().as_millis() as u32;

        // * Send frame to all connected clients
        for client in &self.clients {
    
            // * Frames are send on packets chunk size
            let chunk_size = Packet::CHUNK_SIZE;

            // chunk_size - 1 because the first byte is the index
            for (i, chunk) in bytes.chunks(chunk_size - Packet::META_SIZE).enumerate() {

                let packet = Packet {
                    index: i as u8,
                    frame_id,
                    data: chunk.to_vec(),
                };

                match self.listener.send_to(&packet.to_bytes(), client) {
                    Ok(bytes_send) => {
                        // println!("\nPacket {} : size {}", i, bytes_send);
                    }
                    Err(e) => {
                        println!("Error sending packet to client: {}", e);
                        clients_to_remove.push(*client);
                        break;
                    }
                }
            }

        }

        // * Remove clients with errors
        for client in clients_to_remove {
            self.clients.retain(|&x| x != client);
        }

        // * This code Stops the capture after seconds
        // if self.start.elapsed().as_secs() >= 10 {
        //     self.encoder.take().unwrap().finish()?;
        //     capture_control.stop();
        //     println!("REACHED STREAMING LIMIT");
        // }

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) closes.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture Session Closed");
        Ok(())
    }
}

#[tokio::main]
pub async fn run(options: commands::StartCmd) {
    // Gets The Foreground Window, Checkout The Docs For Other Capture Items
    let primary_monitor = Monitor::primary().expect("There is no primary monitor");

    let settings = Settings::new(
        // Item To Captue
        primary_monitor,
        // Capture Cursor Settings
        CursorCaptureSettings::Default,
        // Draw Borders Settings
        DrawBorderSettings::Default,
        // The desired color format for the captured frame.
        ColorFormat::Rgba8,
        // Additional flags for the capture settings that will be passed to user defined `new` function.
        options.as_string(),
    )
    .unwrap();

    // Starts the capture and takes control of the current thread.
    // The errors from handler trait will end up here
    match Capture::start(settings) {
        Ok(_) => println!("Capture finished"),
        Err(e) => eprintln!("Error when capturing {}", e),
    }
    // Stop the server
    exit(0);
}
