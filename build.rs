/*
 * This build script allows to link the assembly and C language part of the kernel to the rest of the code.
 */

/*
 * The entry of the build script for the kernel.
 */
fn main() {
    println!("cargo:rustc-link-search=native=./");
    println!("cargo:rustc-link-lib=static=maestro");
    println!("cargo:rerun-if-changed=libmaestro.a");
}
