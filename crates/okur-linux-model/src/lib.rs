//! Linux's accessibility, keyboard and speech data as data, shared by the Linux driver
//! (`okur-linux`) and replay: nothing here calls Linux, so it builds and is tested on every OS.

pub mod keys;
pub mod ssip;
