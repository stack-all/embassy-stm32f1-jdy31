fn main() {
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    // Set DEFMT_LOG=info through environment variables to enable probe rs to support defmt: Output of info level log for info
    // println!("cargo:rustc-env=DEFMT_LOG=info");
}