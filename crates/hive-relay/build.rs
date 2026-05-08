fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use vendored protoc so builds work without system protoc installed.
    let protoc = protoc_bin_vendored::protoc_bin_path()
        .map_err(|e| format!("protoc-bin-vendored: {e}"))?;
    std::env::set_var("PROTOC", protoc);
    tonic_build::compile_protos("proto/hive.proto")?;
    Ok(())
}
