use cargo_metadata::MetadataCommand;
use std::env;

fn main() {
    // Get the target directory from cargo
    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let target_dir = metadata.target_directory;

    // Determine the profile (debug or release)
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Get target and host triples
    let target_triple = env::var("TARGET").ok();
    let host_triple = env::var("HOST").ok();

    // Construct the path to the coreutils binary
    let binary_name = if cfg!(windows) {
        "coreutils.exe"
    } else {
        "coreutils"
    };

    // Build the binary path based on whether we're cross-compiling
    let binary_path = if target_triple != host_triple {
        // For cross compilation: target/<target-triple>/debug/coreutils
        target_dir
            .join(target_triple.unwrap())
            .join(&profile)
            .join(binary_name)
    } else {
        // For native compilation: target/debug/coreutils
        target_dir.join(&profile).join(binary_name)
    };

    // Output the binary path for use in the tests
    println!("cargo:rustc-env=CARGO_BIN_EXE_coreutils={}", binary_path);
}
