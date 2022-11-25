fn main() -> Result<(), Box<dyn std::error::Error>> {
    // sets some build env vars
    // this allows the binary to know which commit it was built from, among other things
    cli_batteries::build_rs().unwrap();

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(
            &[
                "schemas/protobufs/sitter.proto",
                "schemas/protobufs/admin.proto",
            ],
            &[""],
        )?;

    println!("cargo:rerun-if-changed=schemas/protobufs/sitter.proto");
    println!("cargo:rerun-if-changed=schemas/protobufs/admin.proto");

    println!("cargo:rerun-if-changed=schemas/database");

    Ok(())
}
