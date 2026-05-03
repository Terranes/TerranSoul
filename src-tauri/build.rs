fn main() {
    let brain_proto = "proto/terransoul/brain.v1.proto";
    let phone_proto = "proto/terransoul/phone_control.v1.proto";
    println!("cargo:rerun-if-changed={brain_proto}");
    println!("cargo:rerun-if-changed={phone_proto}");
    if let Ok(protoc) = protoc_bin_vendored::protoc_bin_path() {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_prost_build::compile_protos(brain_proto).expect("compile brain.v1.proto");
    tonic_prost_build::compile_protos(phone_proto).expect("compile phone_control.v1.proto");
    tauri_build::build()
}
