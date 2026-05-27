fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("outlayer_descriptor.bin"))
        .compile_protos(&["proto/outlayer.proto"], &["proto"])?;
    Ok(())
}
