//! PS4 PKG parsing (DESIGN 9.5).

pub mod fixture;
pub mod header;
pub mod scan;
pub mod sfo;

pub use header::{Icon, Kind, PkgError, PkgInfo, read_info};
