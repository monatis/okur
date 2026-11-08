//! Linux's accessibility, keyboard and speech data as data, shared by the Linux driver
//! (`okur-linux`) and replay: nothing here calls Linux, so it builds and is tested on every OS.

pub mod convert;
pub mod delivery;
pub mod focus;
pub mod focus_track;
pub mod keys;
pub mod page;
pub mod ssip;
