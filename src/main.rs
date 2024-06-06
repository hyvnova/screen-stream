use clap::Parser;

mod client;
pub mod packet;
mod server;
pub mod frame_buffer;
pub mod commands;
pub mod comm;
pub mod shared;

use commands::Cmds;

#[derive(Parser)]
#[command(
    version = "1.0",
    about = "Screen Stream",
    long_about = "Stream your screen by ip address and port.",
    author = "ezsnova"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmds,
}

#[tokio::main]
async fn main() {
    
    let cli = Cli::parse();

    match cli.cmd {
        Cmds::Start(options) => {
            server::run(options).await;
        }

        Cmds::Connect(connect) => {
            let _ = client::run(connect.address);
        }
    }
}
