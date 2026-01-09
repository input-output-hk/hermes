//! Build
fn main() {
    built::write_built_file().expect("should acquire build-time information");
}
