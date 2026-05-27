#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

tonic::include_proto!("outlayer");

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("outlayer_descriptor");
