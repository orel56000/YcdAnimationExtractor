//! YCD clip dictionary reader (GTA V) — Rust port of `ycd-scanner`.

pub mod resource_reader;
pub mod ycd_parse;

pub use ycd_parse::{decompress_ycd_buffer, parse_ycd_animations, to_short_name, ParseResult};
