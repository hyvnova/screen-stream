use clap::{Args, Parser, Subcommand};

mod client;
pub mod packet;
mod server;

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

#[derive(Subcommand)]
enum Cmds {
    #[command(about = "Start streaming server")]
    Start(StartCmd), // ss start -p <port | default = 8080>
    #[command(about = "Connect to a (streaming) server")]
    Connect(ConnectCmd), // ss connect -a <ip>:<port>
}

#[derive(Args)]
struct StartCmd {
    #[arg(short, long, default_value = "8080")]
    port: u16,
}

#[derive(Args)]
struct ConnectCmd {
    address: String,
}

fn main() {
    let cli = Cli::parse();

    match cli.cmd {
        Cmds::Start(start) => {
            server::run(start.port);
        }

        Cmds::Connect(connect) => {
            let _ = client::run(connect.address);
        }
    }
}
