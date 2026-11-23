fn main() {
    unsafe {
        std::env::set_var(
            "PROTOC",
            protoc_bin_vendored::protoc_bin_path().expect("vendored protoc binary path"),
        );
    }

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let mut config = prost_build::Config::new();
    config.enable_type_names();

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("greeter_descriptor.bin"))
        .compile_protos_with_config(
            config,
            &[
                "proto/biyard/greeter/v1/greeter.proto",
                "proto/biyard/greeter/v2/greeter.proto",
            ],
            &["proto"],
        )
        .expect("tonic-build: compile greeter protos");

    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=build.rs");
}
