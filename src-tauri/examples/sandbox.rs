//! Generates the demo repository at the given path.
//!
//! cargo run --example sandbox -- /tmp/relay-demo

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args().nth(1).ok_or("usage: sandbox <directory>")?;
    graft_lib::demo::sandbox::generate(std::path::Path::new(&root))?;
    println!("demo repository written to {root}");
    Ok(())
}
