# Rust Kernel Development Recap

## Goal

Build a minimal freestanding Rust kernel that can eventually boot in QEMU.

---

# 1. Updating Rust Toolchain

Initial error:

```txt
feature `edition2024` is required
```

Cause:
- Cargo version too old for Rust Edition 2024.

Fix:

```bash
rustup update
```

---

# 2. Deprecated `rls-preview`

Update failed because old RLS component was installed.

Fix:

```bash
rustup component remove rls-preview
```

Modern Rust uses:

- rust-analyzer

instead of RLS.

---

# 3. MSVC Linker Error

Error:

```txt
link.exe not found
```

MSVC toolchain requires Visual Studio linker.

Instead of fixing MSVC initially, switched to GNU toolchain.

Fix:

```bash
rustup default stable-x86_64-pc-windows-gnu
```

---

# 4. First Successful Build

After switching to GNU:

```bash
cargo build
```

worked successfully.

---

# 5. Creating a Freestanding Binary

Started converting normal Rust program into a kernel-style binary.

Used:

```rust
#![no_std]
#![no_main]
```

Meaning:

- no standard library
- no runtime
- no OS dependencies

---

# 6. Panic Handler

Added custom panic handler:

```rust
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```

---

# 7. Custom Entry Point

Replaced `main()` with:

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
```

This became the kernel entry point.

---

# 8. Panic Unwinding Error

Error:

```txt
unwinding panics are not supported without std
```

Fix in `Cargo.toml`:

```toml
[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

---

# 9. WinMain Error

Error:

```txt
undefined reference to `WinMain`
```

Cause:
- Still building for Windows target.

Moved toward freestanding target:

```txt
x86_64-unknown-none
```

---

# 10. Using Nightly Rust

`build-std` requires nightly.

Installed nightly GNU toolchain:

```bash
rustup override set nightly-x86_64-pc-windows-gnu
```

---

# 11. Installing rust-src

Needed Rust source code for rebuilding core libraries.

Installed:

```bash
rustup component add rust-src
```

---

# 12. Building core + compiler_builtins

Cargo started rebuilding:

- core
- compiler_builtins

for freestanding environment.

This is required for kernels because there is no OS runtime.

---

# 13. VGA Text Output Kernel

Created minimal VGA text writer:

```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    unsafe {
        *vga_buffer.offset(0) = b'H';
        *vga_buffer.offset(1) = 0x0f;

        *vga_buffer.offset(2) = b'i';
        *vga_buffer.offset(3) = 0x0f;
    }

    loop {}
}
```

This writes directly into VGA memory.

---

# 14. Installing QEMU

Installed QEMU emulator.

Verified:

```bash
qemu-system-x86_64 --version
```

Needed to add QEMU directory to PATH.

Temporary fix:

```bash
set PATH=%PATH%;C:\Users\manic\QEMU
```

---

# 15. Installing bootimage

Installed bootimage tool:

```bash
cargo install bootimage
```

Purpose:
- Creates bootable disk image from kernel.

---

# 16. Installing LLVM Tools

Installed LLVM utilities:

```bash
rustup component add llvm-tools-preview
```

Used internally by bootimage.

---

# 17. Creating Bootable Images

Used:

```bash
cargo bootimage
```

This process:

1. builds kernel
2. builds bootloader
3. packages bootable image

Result:

```txt
bootimage-os.bin
```

---

# 18. Bootloader Dependency

Needed bootloader dependency.

Added to `Cargo.toml`:

```toml
[dependencies]
bootloader = "0.9.29"
```

---

# 19. Page Mapping Panic

Encountered:

```txt
failed to map page
PageAlreadyMapped
```

Cause:
- old bootloader + new nightly incompatibility.

Fix:
- pinned older nightly:

```bash
rustup toolchain install nightly-2024-01-01
```

---

# 20. Edition 2024 Compatibility

Older nightly did not fully support Edition 2024.

Changed:

```toml
edition = "2024"
```

to:

```toml
edition = "2021"
```

---

# 21. Cargo.lock Compatibility

Error:

```txt
lock file version 4 requires -Znext-lockfile-bump
```

Fix:

```bash
del Cargo.lock
```

Then Cargo regenerated compatible lockfile.

---

# 22. Older Nightly Attribute Syntax

Error:

```txt
expected identifier, found keyword `unsafe`
```

Cause:
- older nightly doesn't support:

```rust
#[unsafe(no_mangle)]
```

Fix:

```rust
#[no_mangle]
```

---

# 23. rust-lld Crash

Error:

```txt
rust-lld failed: exit code: 0xc0000374
```

Meaning:
- linker heap corruption crash.

Likely caused by:
- Windows GNU toolchain instability
- old nightly + linker combination

Recommended future path:
- use MSVC nightly with Visual Studio Native Tools terminal.

---

# Important Concepts Learned

## Freestanding Binary

A binary without:
- operating system
- standard library
- runtime

---

## no_std

Removes Rust standard library.

---

## no_main

Removes normal Rust runtime startup.

---

## _start

Real entry point for kernel execution.

---

## VGA Buffer

Memory-mapped text mode at:

```txt
0xb8000
```

Allows direct hardware text output.

---

## Bootloader

Responsible for:
- loading kernel into memory
- setting up CPU state
- jumping to kernel entry point

---

## QEMU

Acts like a virtual x86_64 computer.

Provides:
- virtual CPU
- RAM
- VGA hardware
- BIOS emulation

---

# Current Architecture

```txt
Rust Source
    ↓
Freestanding Kernel Binary
    ↓
Bootloader
    ↓
Bootable Disk Image
    ↓
QEMU Virtual Machine
    ↓
Virtual Hardware Execution
```

---

# What Comes Next

Future kernel topics:

- VGA text driver
- interrupts
- GDT / IDT
- memory allocator
- paging
- keyboard driver
- heap allocation
- scheduler
- processes
- syscalls
- filesystem
- multitasking

---

# Useful Commands Recap

## Build kernel

```bash
cargo build -Z build-std=core,compiler_builtins
```

## Create bootable image

```bash
cargo bootimage
```

## Run boot image in QEMU

```bash
qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/debug/bootimage-os.bin
```

## Verify QEMU

```bash
qemu-system-x86_64 --version
```

## Install rust-src

```bash
rustup component add rust-src
```

## Install LLVM tools

```bash
rustup component add llvm-tools-preview
```

## Install bootimage

```bash
cargo install bootimage
```
