use std::env;
use std::path::Path;
// 删除未使用的导入: use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // 获取目标平台信息
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:warning=Building for target OS: {}", target_os);
    println!("cargo:warning=Building for target architecture: {}", target_arch);

    // 检查是否启用了静态链接
    let static_link = env::var("CARGO_FEATURE_STATIC_LINK").is_ok();
    if static_link {
        println!("cargo:warning=Static linking enabled");

        // 为不同平台设置静态链接标志
        if target_os == "windows" {
            // 检查是否使用 MSVC 工具链
            let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_else(|_| "unknown".to_string());
            
            if target_env == "msvc" {
                // MSVC 工具链使用 /NODEFAULTLIB 标志
                println!("cargo:rustc-link-arg=/NODEFAULTLIB:libcmt.lib");
                println!("cargo:rustc-link-arg=/NODEFAULTLIB:msvcrt.lib");
                println!("cargo:rustc-link-arg=/NODEFAULTLIB:msvcrtd.lib");
            } else {
                // MinGW 工具链使用不同的标志
                println!("cargo:rustc-link-arg=-static");
                println!("cargo:rustc-link-arg=-static-libgcc");
                println!("cargo:rustc-link-arg=-static-libstdc++");
            }
        } else if target_os == "linux" {
            println!("cargo:rustc-link-arg=-static-libgcc");
            println!("cargo:rustc-link-arg=-static-libstdc++");
        }
    }

    // 检查是否启用了 hyperscan_engine 特性
    let is_hyperscan_enabled = env::var("CARGO_FEATURE_HYPERSCAN_ENGINE").is_ok();

    if is_hyperscan_enabled {
        println!("cargo:rustc-cfg=feature=\"hyperscan_engine\"");

        // 检查 hyperscan 是否可用
        // 注意：交叉编译时，我们不能检查主机系统上的库
        // 因此，我们只在目标平台与主机平台相同时才检查
        let check_host = match target_os.as_str() {
            "macos" => cfg!(target_os = "macos"),
            "linux" => cfg!(target_os = "linux"),
            "windows" => cfg!(target_os = "windows"),
            _ => false,
        };

        if check_host {
            let has_hyperscan = check_hyperscan_available(&target_os);

            if has_hyperscan {
                println!("cargo:warning=Using Hyperscan for regex matching acceleration");
            } else {
                println!("cargo:warning=Hyperscan library not found");
                println!("cargo:warning=Hyperscan can be installed from https://www.hyperscan.io/");
                println!("cargo:warning=Falling back to standard regex engine");

                // 平台特定的安装说明
                if target_os == "macos" {
                    println!("cargo:warning=On macOS, try: brew install hyperscan");
                } else if target_os == "linux" {
                    println!("cargo:warning=On Ubuntu/Debian, try: sudo apt-get install libhyperscan-dev");
                    println!("cargo:warning=On Fedora/RHEL, try: sudo dnf install hyperscan-devel");
                } else if target_os == "windows" {
                    println!("cargo:warning=On Windows, build from source: https://github.com/intel/hyperscan");
                }

                // 禁用 hyperscan_engine 特性
                println!("cargo:rustc-cfg=feature=\"no_hyperscan\"");
            }
        } else {
            println!("cargo:warning=Cross-compiling for {}, skipping Hyperscan check", target_os);
            println!("cargo:warning=Falling back to standard regex engine");
            println!("cargo:rustc-cfg=feature=\"no_hyperscan\"");
        }
    } else {
        println!("cargo:warning=Building without Hyperscan acceleration");
        println!("cargo:warning=Enable with: cargo build --features hyperscan_engine");
    }

    // 设置版本信息
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    println!("cargo:rustc-env=APP_VERSION={}", version);

    // 设置构建时间
    let build_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
}

fn check_hyperscan_available(target_os: &str) -> bool {
    // 尝试使用 pkg-config 检测 hyperscan
    if let Ok(lib) = pkg_config::probe_library("libhs") {
        println!("cargo:warning=Found Hyperscan at {:?}", lib.include_paths);
        return true;
    }

    // 替代检测方法 - 直接检查库文件
    let lib_found = match target_os {
        "windows" => {
            // 检查 Windows 风格的库 (DLL)
            check_lib_exists("hs.dll")
        },
        "macos" => {
            // 检查 macOS 风格的库
            check_lib_exists("libhs.dylib") ||
            check_lib_exists("/usr/local/lib/libhs.dylib") ||
            check_lib_exists("/opt/homebrew/lib/libhs.dylib")
        },
        "linux" => {
            // 检查 Linux 风格的库
            check_lib_exists("libhs.so") ||
            check_lib_exists("/usr/lib/libhs.so") ||
            check_lib_exists("/usr/local/lib/libhs.so") ||
            check_lib_exists("/usr/lib/x86_64-linux-gnu/libhs.so")
        },
        _ => false,
    };

    lib_found
}

fn check_lib_exists(lib_path: &str) -> bool {
    Path::new(lib_path).exists()
}
