fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let mut config = tonic_prost_build::Config::new();
    config
        .protoc_executable(protoc_bin_vendored::protoc_bin_path().expect("vendored protoc"))
        .enable_type_names();

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("greeter_descriptor.bin"))
        .compile_with_config(
            config,
            &[
                "proto/biyard/greeter/v1/greeter.proto",
                "proto/biyard/greeter/v2/greeter.proto",
            ],
            &["proto"],
        )
        .expect("compile greeter protos");

    println!("cargo:rerun-if-changed=proto");
}
