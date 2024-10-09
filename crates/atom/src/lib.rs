#[deny(missing_docs)]
mod core;
mod id;

pub mod manifest;
pub mod publish;
pub mod store;
pub mod uri;
pub use core::Atom;
pub use id::AtomId;
pub use id::CalculateRoot;
pub use manifest::Manifest;

const BASE32: base32::Alphabet = base32::Alphabet::Rfc4648HexLower { padding: false };
