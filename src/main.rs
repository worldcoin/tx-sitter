use cli_batteries::{run, version};

fn main() {
    run(version!(), tx_sitter::app);
}
