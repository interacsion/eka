#[deny(missing_docs)]
mod core;
mod id;

pub mod manifest;
pub mod publish;
pub mod uri;
pub use core::Atom;
pub use id::AtomId;
pub use manifest::Manifest;
