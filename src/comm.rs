
/// Communication of server and clients
pub enum Actions {
    // * Unknown action
    Unknown,

    // * Ping - Client to server to know if server is still alive
    Ping = 1,

    // * New connection - Client to server to notify of new connection
    NewConnection = 2,

    // * Disconnection - Client to server to notify of disconnection
    Disconnection = 3,

    // * Request frame - Client to server to request for frame  
    RequestFrame = 4,
    
    // * General ok 
    GeneralOk = 5,
} 

impl From<u8> for Actions {
    fn from(value: u8) -> Self {
        match value {
            1 => Actions::Ping,
            2 => Actions::NewConnection,
            3 => Actions::Disconnection,
            4 => Actions::RequestFrame,
            5 => Actions::GeneralOk,
            _ => Actions::Unknown,
        }
    }
}