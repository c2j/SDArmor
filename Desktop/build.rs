use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // Check if we're building with hyperscan_engine feature
    let is_hyperscan_enabled = env::var("CARGO_FEATURE_HYPERSCAN_ENGINE").is_ok();
    
    if is_hyperscan_enabled {
        println!("cargo:rustc-cfg=feature=\"hyperscan_engine\"");
        
        // Check if hyperscan is available
        let has_hyperscan = check_hyperscan_available();
        
        if has_hyperscan {
            println!("cargo:warning=Using Hyperscan for regex matching acceleration");
        } else {
            println!("cargo:warning=Hyperscan library not found");
            println!("cargo:warning=Hyperscan can be installed from https://www.hyperscan.io/");
            println!("cargo:warning=Falling back to standard regex engine");
            
            // Platform-specific installation instructions
            if cfg!(target_os = "macos") {
                println!("cargo:warning=On macOS, try: brew install hyperscan");
            } else if cfg!(target_os = "linux") {
                println!("cargo:warning=On Ubuntu/Debian, try: sudo apt-get install libhyperscan-dev");
                println!("cargo:warning=On Fedora/RHEL, try: sudo dnf install hyperscan-devel");
            } else if cfg!(target_os = "windows") {
                println!("cargo:warning=On Windows, build from source: https://github.com/intel/hyperscan");
            }
            
            // Disable the hyperscan_engine feature
            println!("cargo:rustc-cfg=feature=\"no_hyperscan\"");
        }
    } else {
        println!("cargo:warning=Building without Hyperscan acceleration");
        println!("cargo:warning=Enable with: cargo build --features hyperscan_engine");
    }
    
    // We no longer need to generate build-time information
}

fn check_hyperscan_available() -> bool {
    // Try using pkg-config to detect hyperscan
    if let Ok(lib) = pkg_config::probe_library("libhs") {
        println!("cargo:warning=Found Hyperscan at {:?}", lib.include_paths);
        return true;
    }
    
    // Alternative detection method - check for library directly
    let lib_found = if cfg!(target_os = "windows") {
        // Check for Windows-style library (DLL)
        check_lib_exists("hs.dll")
    } else if cfg!(target_os = "macos") {
        // Check for macOS-style library
        check_lib_exists("libhs.dylib") || check_lib_exists("/usr/local/lib/libhs.dylib")
    } else {
        // Check for Linux-style library
        check_lib_exists("libhs.so") || check_lib_exists("/usr/lib/libhs.so") || 
            check_lib_exists("/usr/local/lib/libhs.so")
    };
    
    lib_found
}

fn check_lib_exists(lib_path: &str) -> bool {
    Path::new(lib_path).exists()
}

// Function removed as we no longer need to generate build info