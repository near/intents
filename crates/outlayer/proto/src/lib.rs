#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

tonic::include_proto!("outlayer");

pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/outlayer_descriptor.bin"));
