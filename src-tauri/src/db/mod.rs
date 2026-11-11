pub mod connection;
pub mod evaluations;
pub mod jobs;
pub mod migrations;
pub mod profile;
pub mod runs;
pub mod sources;

pub use connection::Db;

use chrono::Utc;

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}
