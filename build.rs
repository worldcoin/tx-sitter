fn main() {
    // sets some build env vars
    // this allows the binary to know which commit it was built from, among other things
    cli_batteries::build_rs().unwrap();

    println!("cargo:rerun-if-changed=migrations");
}
