use std::{ops::Deref, sync::{Arc, Mutex, MutexGuard}};

pub struct Shared<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> Shared<T> {
    /// Create a new Shared instance
    pub fn new(value: T) -> Shared<T> {
        Shared {
            inner: Arc::new(Mutex::new(value))
        }   
    }

    /// Consume the value inside the shared value
    pub fn consume(&self) -> MutexGuard<T> {
        return self.inner.lock().unwrap();
    }
}


impl<T> Deref for Shared<T> {
    type Target = Arc<Mutex<T>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared {
            inner: self.inner.clone(),
        }
    }
}


#[macro_export]
/// Create a Arc<Mutex<T>> from a value
/// Why shard? because it's a shared value.
/// Nah, it's because it sounds cool
macro_rules! shard {
    ($x:expr) => {
        crate::shared::Shared::new($x)
    };
}

// Re-export the shard macro
pub use crate::shard;
