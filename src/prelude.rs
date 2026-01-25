use std::time::SystemTime;

pub trait PathHelpers {
    fn modtime(&self) -> Option<SystemTime>;
}
