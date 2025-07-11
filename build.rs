use std::env;

fn main() {
    // 获取目标平台信息
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TARGET");

    // 根据平台设置编译标志
    match target_os.as_str() {
        "windows" => {
            println!("cargo:rustc-cfg=platform_windows");
            // Windows 特定的构建配置
            if target_arch == "x86_64" {
                println!("cargo:rustc-cfg=arch_x64");
            } else if target_arch == "x86" {
                println!("cargo:rustc-cfg=arch_x86");
            }
        }
        "macos" => {
            println!("cargo:rustc-cfg=platform_macos");
            // macOS 特定的构建配置
            if target_arch == "x86_64" {
                println!("cargo:rustc-cfg=arch_x64");
            } else if target_arch == "aarch64" {
                println!("cargo:rustc-cfg=arch_arm64");
            }
        }
        "linux" => {
            println!("cargo:rustc-cfg=platform_linux");
            // Linux 特定的构建配置
            if target_arch == "x86_64" {
                println!("cargo:rustc-cfg=arch_x64");
            } else if target_arch == "aarch64" {
                println!("cargo:rustc-cfg=arch_arm64");
            }
        }
        _ => {
            println!("cargo:rustc-cfg=platform_unknown");
        }
    }

    // 输出构建信息
    println!("cargo:warning=Building for target: {}", target);
    println!("cargo:warning=Target OS: {}", target_os);
    println!("cargo:warning=Target Architecture: {}", target_arch);
}
