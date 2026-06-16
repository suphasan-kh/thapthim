fn main() {
    // This tells Cargo to rebuild the Rust code automatically 
    // only if you modify files inside the src/ directory.
    println!("cargo:rerun-if-changed=src");
}