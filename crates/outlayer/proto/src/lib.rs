#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

tonic::include_proto!("outlayer");

/// Binary-encoded protobuf schema (a [`FileDescriptorSet`]) for this package.
/// Used by the tonic/gRPC server reflection protocol to let clients
/// auto-discover services, methods, and messages at runtime without having the
/// `.proto` file on hand.
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("outlayer_descriptor");
