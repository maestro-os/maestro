/*
 * The entry of the build script for the library.
 */
fn main() {
    println!("cargo:rustc-link-search=native=./");
    println!("cargo:rustc-link-lib=static=util");
    println!("cargo:rerun-if-changed=libutil.a");
}
