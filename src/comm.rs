
/// Communication of server and clients
pub enum Actions {
    // * Unknown action
    Unknown = -1,

    // * Ping - Client to server to know if server is still alive
    Ping = 1,

    // * New connection - Client to server to notify of new connection
    NewConnection = 2,

    // * Disconnection - Client to server to notify of disconnection
    Disconnection = 3,

} 

impl From<u8> for Actions {
    fn from(value: u8) -> Self {
        match value {
            1 => Actions::Ping,
            2 => Actions::NewConnection,
            3 => Actions::Disconnection,
            _ => Actions::Unknown,
        }
    }
}