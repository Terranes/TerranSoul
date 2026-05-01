fn main() {
    let proto = "proto/terransoul/brain.v1.proto";
    println!("cargo:rerun-if-changed={proto}");
    if let Ok(protoc) = protoc_bin_vendored::protoc_bin_path() {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_prost_build::compile_protos(proto).expect("compile brain.v1.proto");
    tauri_build::build()
}
