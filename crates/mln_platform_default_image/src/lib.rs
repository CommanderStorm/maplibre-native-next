//! Ports the platform-default image codecs declared in
//! `include/mln/util/image.hpp` (`decodeImage`, `encodePNG`):
//! `platform/default/src/mln/util/{image,png_reader,png_writer}.cpp`, plus the
//! `util::compress` they use from `compression.cpp`.
//!
//! C++ throws `std::runtime_error`; here the functions return
//! `Err(message)` with the same message.

pub mod compression;
pub mod image;
pub mod png_reader;
pub mod png_writer;
pub mod webp_reader;

pub use image::decode_image;
pub use png_reader::decode_png;
pub use png_writer::encode_png;
