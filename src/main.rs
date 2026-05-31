use std::process::Command;
use std::path::Path;

fn main() {
    // Load environment variables from .env file if present
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = trimmed.split_once('=') {
                std::env::set_var(key.trim(), val.trim());
            }
        }
    }

    // 1. Build the kernel
    println!("Building kernel...");
    let mut build_cmd = Command::new("cargo");
    build_cmd.args([
        "build",
        "--release", // Compile with release profile optimizations
        "--package",
        "kernel",
        "--target",
        "x86_64-unknown-none",
        "-Z",
        "build-std=core,compiler_builtins",
    ]);

    let status = build_cmd.status().expect("failed to run cargo build for kernel");
    if !status.success() {
        eprintln!("Kernel build failed");
        std::process::exit(1);
    }

    // 2. Locate compiled kernel ELF
    let kernel_elf = Path::new("target/x86_64-unknown-none/release/kernel");
    if !kernel_elf.exists() {
        eprintln!("Kernel ELF not found at {:?}", kernel_elf);
        std::process::exit(1);
    }

    // 3. Create bootable BIOS image
    println!("Packaging boot image...");
    let bios_path = Path::new("target/x86_64-unknown-none/release/bios.img");
    bootloader::BiosBoot
        ::new(kernel_elf)
        .create_disk_image(bios_path)
        .expect("failed to create BIOS disk image");

    // 4. Run QEMU
    println!("Launching QEMU...");
    let mut qemu_candidates = Vec::new();
    if let Ok(env_qemu) = std::env::var("QEMU_PATH") {
        qemu_candidates.push(env_qemu);
    }
    qemu_candidates.push("qemu-system-x86_64".to_string());
    qemu_candidates.push("C:\\Program Files\\qemu\\qemu-system-x86_64.exe".to_string());
    qemu_candidates.push("C:\\Program Files (x86)\\qemu\\qemu-system-x86_64.exe".to_string());

    let mut qemu_cmd = None;
    for candidate in &qemu_candidates {
        let test_cmd = if candidate == "qemu-system-x86_64" {
            // Check if it's in the PATH
            Command::new(candidate).arg("--version").output().is_ok()
        } else {
            Path::new(candidate).exists()
        };
        if test_cmd {
            qemu_cmd = Some(Command::new(candidate));
            break;
        }
    }

    // Get absolute path to bios image and strip UNC prefix if present
    let mut absolute_bios_path = std::fs
        ::canonicalize(bios_path)
        .expect("failed to get absolute path of bios.img")
        .display()
        .to_string();
    if absolute_bios_path.starts_with(r"\\?\") {
        absolute_bios_path = absolute_bios_path[4..].to_string();
    }

    let mut cmd = qemu_cmd.expect(
        "failed to find QEMU. Ensure QEMU is installed or specify its path in QEMU_PATH in the .env file"
    );

    // Set path environment to resolve sibling DLLs for Android SDK emulator QEMU
    let program = cmd.get_program().to_string_lossy().into_owned();
    if program.contains("Android") && program.contains("emulator") {
        if let Some(parent) = Path::new(&program).parent() {
            if let Some(grandparent) = parent.parent() {
                if let Some(great_grandparent) = grandparent.parent() {
                    let lib64 = great_grandparent.join("lib64");
                    let qt_lib = lib64.join("qt").join("lib");

                    // Prepend parent, grandparent, great_grandparent, lib64, qt_lib, and UCRT downlevel directory to PATH
                    let mut new_path = format!(
                        "{};{};{};{};{};C:\\Windows\\System32\\downlevel",
                        parent.display(),
                        grandparent.display(),
                        great_grandparent.display(),
                        lib64.display(),
                        qt_lib.display()
                    );

                    let path_key = std::env
                        ::vars()
                        .map(|(k, _)| k)
                        .find(|k| k.eq_ignore_ascii_case("PATH"))
                        .unwrap_or_else(|| "PATH".to_string());

                    let mut current_path = String::new();
                    for (key, val) in std::env::vars() {
                        if key.eq_ignore_ascii_case("path") {
                            cmd.env_remove(&key);
                            if current_path.is_empty() {
                                current_path = val;
                            }
                        }
                    }

                    if !current_path.is_empty() {
                        new_path = format!("{};{}", new_path, current_path);
                    }
                    cmd.env(path_key, new_path);
                }
            }
        }
    }

    cmd.arg("-drive").arg(format!("format=raw,file={}", absolute_bios_path));
    println!("Running QEMU command: {:?}", cmd);
    let mut child = cmd.spawn().expect("failed to start QEMU");
    let exit_status = child.wait().unwrap();

    if !exit_status.success() {
        std::process::exit(exit_status.code().unwrap_or(1));
    }
}
