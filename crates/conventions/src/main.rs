//! Writes and checks Augentic's shared conventions in a consuming repository.
//!
//! The program arrives with the `conventions/` tree; this is the workspace
//! member it is built from.

fn main() {
    println!("conventions {}", env!("CARGO_PKG_VERSION"));
}
