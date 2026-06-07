//! Synchronization Primitives and Thread-safe Data Structures

pub(crate) mod primitives {
    pub(crate) mod mutex;
}

pub(crate) use primitives::mutex::Mutex;
