use clap::{Args, Subcommand};



#[derive(Subcommand)]
pub enum Cmds {
    #[command(about = "Start streaming server")]
    Start(StartCmd), // ss start -p <port | default = 8080>
    #[command(about = "Connect to a (streaming) server")]
    Connect(ConnectCmd), // ss connect -a <ip>:<port>
}

#[derive(Args)]
pub struct StartCmd {
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    #[arg(short, long, default_value = "25", help = "Quality of the stream")]
    pub quality: u8, 

    #[arg(short, long, help="Resolution of the stream", default_value="1920x1080")]
    pub resolution: String, 
}

impl StartCmd {
    pub fn as_string(&self) -> String {
        format!("{}:{}:{}", self.port, self.quality, self.resolution)
    }

    pub fn parse(s: String) -> StartCmd {
        let mut parts = s.split(':');
        let port = parts.next().unwrap().parse().unwrap();
        let quality = parts.next().unwrap().parse().unwrap();
        let resolution = parts.next().unwrap().to_string();
        StartCmd { port, quality, resolution }
    }
    
}

#[derive(Args)]
pub struct ConnectCmd {
    pub address: String,
}