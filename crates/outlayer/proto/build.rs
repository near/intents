fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Cargo-provided build output dir. We need it explicitly because prost writes the
    // descriptor set to a literal path (it doesn't prepend OUT_DIR), yet the
    // `include_file_descriptor_set!` macro reads from OUT_DIR — so we must point prost there.
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    tonic_prost_build::configure()
        // Emit the encoded FileDescriptorSet so gRPC reflection works. prost writes it
        // to this exact path, while `include_file_descriptor_set!("outlayer_descriptor")`
        // (in lib.rs) reads from `$OUT_DIR/outlayer_descriptor.bin` — so the file must
        // live in OUT_DIR with that stem, which is why we join OUT_DIR here.
        .file_descriptor_set_path(out_dir.join("outlayer_descriptor.bin"))
        // Generate Rust for these protos. The `.rs` output defaults to OUT_DIR and is
        // pulled in by `include_proto!("outlayer")`. Second arg is the include path root.
        .compile_protos(&["proto/outlayer.proto"], &["proto"])?;
    Ok(())
}
