//! Named colors shared by Lisp faces and image decoders.
//!
//! Generated from `etc/rgb.txt` at build time; lookup requires no runtime I/O.

include!(concat!(env!("OUT_DIR"), "/x11_colors.rs"));

#[cfg(test)]
#[path = "x11_colors_test.rs"]
mod tests;
