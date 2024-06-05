use std::sync::{Arc, Mutex};

pub type Shared<T> = Arc<Mutex<T>>;

#[macro_export]
/// Create a Arc<Mutex<T>> from a value
/// Why shard? because it's a shared value.
/// Nah, it's because it sounds cool
macro_rules! shard {
    ($x:expr) => {
        Arc::new(Mutex::new($x))
    }; 
}

// Re-export the shard macro
pub use crate::shard;