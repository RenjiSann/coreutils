use std::env;
use cargo_metadata::MetadataCommand;

fn main() {
    // Get the target directory from cargo
    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let target_dir = metadata.target_directory;
    
    // Determine the profile (debug or release)
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    
    // Construct the path to the coreutils binary
    let binary_name = if cfg!(windows) { "coreutils.exe" } else { "coreutils" };
    let binary_path = target_dir.join(profile).join(binary_name);
    
    // Output the binary path for use in the tests
    println!("cargo:rustc-env=CARGO_BIN_EXE_coreutils={}", binary_path);
}
