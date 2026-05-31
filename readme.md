# Freestanding Rust OS Kernel

A minimal, freestanding `no_std` x86_64 Rust operating system kernel that boots in QEMU using a modern BIOS bootloader.

```
                  ┌────────────────────────┐
                  │      Rust Source       │
                  └───────────┬────────────┘
                              │
                              ▼
                  ┌────────────────────────┐
                  │  Freestanding Kernel   │
                  │ (x86_64-unknown-none)  │
                  └───────────┬────────────┘
                              │
                              ▼
                  ┌────────────────────────┐
                  │       Bootloader       │
                  │   (bootloader_api)     │
                  └───────────┬────────────┘
                              │
                              ▼
                  ┌────────────────────────┐
                  │  Bootable BIOS Image   │
                  │      (bios.img)        │
                  └───────────┬────────────┘
                              │
                              ▼
                  ┌────────────────────────┐
                  │  QEMU Virtual Machine  │
                  │     (VGA Output)       │
                  └────────────────────────┘
```

---

## 1. Project Architecture

The project is structured as a **Cargo Workspace** consisting of two main crates:

1. **[kernel](file:///D:/rust/kernal/os/kernel)**: The freestanding kernel compiled for bare-metal `x86_64-unknown-none`.
   - Disables the standard library (`#![no_std]`) and normal runtime entry points (`#![no_main]`).
   - Defines a custom panic handler and a bootloader entry point utilizing `bootloader_api`.
   - Writes directly to the VGA text buffer at `0xb8000` to display output.
2. **[runner](file:///D:/rust/kernal/os)** (Workspace Root): The host application that orchestrates building the kernel, packaging it with the bootloader, and launching QEMU.
   - Automatically builds the kernel using `-Z build-std`.
   - Packages the compiled ELF into a raw BIOS disk image.
   - Automatically detects, configures, and launches QEMU.

---

## 2. Prerequisites & Setup

Ensure the following dependencies are installed and configured on your Windows system.

### A. Rust Nightly GNU Toolchain

This project overrides the default toolchain to use the Windows GNU Nightly toolchain.

```bash
rustup override set nightly-x86_64-pc-windows-gnu
```

### B. Required Rust Components & Target

The kernel requires standard library source code (to compile `core` and `compiler_builtins` for bare metal) and cross-compiling components.

```bash
# Required to rebuild core library for the bare-metal target
rustup component add rust-src

# Required for bootloader image packaging (uses llvm-objcopy internally)
rustup component add llvm-tools-preview

# Target required for bootloader building
rustup target add x86_64-unknown-uefi
```

### C. QEMU Emulator

A standalone installation of QEMU is required to boot the image.

> [!TIP]
> Do not use the QEMU binary bundled inside the Android SDK emulator. It contains customized wrappers and dynamic Qt components that will cause DLL initialization crashes (`0xc0000135`).

Install the standard standalone QEMU version via Chocolatey (run in an elevated Administrator shell):

```bash
choco install qemu -y
```

Standard QEMU will be installed under `C:\Program Files\qemu\qemu-system-x86_64.exe`.

---

## 3. Running the Project

The build and run workflows are fully unified and automated. Just execute:

```bash
cargo run
```

This triggers the host runner to automatically:

1. Recompile the **kernel** sub-package for the `x86_64-unknown-none` target.
2. Generate the bootable BIOS image at `target/x86_64-unknown-none/debug/bios.img` using `bootloader_api`.
3. Locate QEMU and launch it with the generated image:

   ```bash
   qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/debug/bios.img
   ```

A QEMU GUI window will pop up showing the kernel's graphical framebuffer output:

```txt
Hello World
```

### D. Environment Configuration (`.env`)

The runner supports configuring custom paths (such as QEMU location) through a `.env` file located in the workspace root. Since this file is git-ignored to prevent pushing local developer configurations to the public repository, a template is provided:

1. Copy [.env.example](file:///D:/rust/kernal/os/.env.example) to `.env`:

   ```bash
   cp .env.example .env
   ```

2. Edit the `.env` file to set your custom QEMU installation path:

   ```ini
   # Path to the QEMU system emulator binary
   QEMU_PATH=C:\Program Files\qemu\qemu-system-x86_64.exe
   ```

If `QEMU_PATH` is specified in the `.env` file, the runner will prioritize it over standard default candidate paths when starting the emulator.

---

## 4. Key Operating System Concepts Used

### `no_std`

Disables the Rust standard library (`std`), which requires operating system features (like threads, file systems, and dynamic memory allocation). Instead, the kernel compiles against the basic `core` and `compiler_builtins` libraries.

### `no_main`

Removes the standard Rust runtime initiation sequence which starts at `main()`. Instead, execution starts directly at the address set by the bootloader (`_start` or entry point defined by `bootloader_api`).

### Custom Panic Handler

Since there is no standard library to print panic messages and unwind the stack, a custom `#[panic_handler]` function must be defined. In case of a panic, this handler puts the CPU into an infinite loop.

### VGA Buffer Mode

VGA text mode is a standard way to write text to the screen. It is mapped to physical address `0xb8000`. By writing characters and style attributes directly to this memory location, we can display text without any graphic drivers.

---

## 5. Troubleshooting & Historical Walkthrough

During development, the following major errors were solved:

- **`link.exe not found`**: Resolved by switching from the MSVC toolchain to the GNU toolchain (`stable-x86_64-pc-windows-gnu`).
- **`unwinding panics are not supported without std`**: Fixed by adding `panic = "abort"` to the dev and release profiles in `Cargo.toml`.
- **`undefined reference to WinMain`**: Resolved by target configuring the kernel to use the freestanding `x86_64-unknown-none` target rather than building for the host OS.
- **`lock file version 4 requires -Znext-lockfile-bump`**: Resolved by deleting `Cargo.lock` and letting Cargo recreate a lockfile version compatible with the active toolchain.
- **QEMU `0xc0000135` (STATUS_DLL_NOT_FOUND)**: Occurred when using Android SDK's QEMU due to missing Qt and wrapper libraries. Resolved by installing standalone QEMU via `choco` and prioritizing it in the launcher candidates.

## 6. Binary Size & Compile Optimizations

    To keep the kernel freestanding binary size as small as possible, the project uses aggressive compiler
  optimization flags configured at the workspace root. These configurations reduce the final compiled kernel ELF from
  **~2.73 MB** (unoptimized debug build) down to **~3.03 KB** (optimized release build)—a **99.88%** reduction in
  footprint.

    ### Key Optimizations Configured:
    - **`panic = "immediate-abort"`**: Configured using `cargo-features = ["panic-immediate-abort"]`. Completely
  removes panic formatting strings and printing machinery, aborting execution immediately in the event of a panic.
    - **`opt-level = "z"`**: Instructs the compiler to optimize the output specifically for minimal binary size.
    - **`lto = true`**: Enables Link-Time Optimization (LTO), allowing optimizations to span across crate dependencies
  (e.g., standard library sources like `core` and `compiler_builtins`).
    - **`codegen-units = 1`**: Compiles the crate as a single unit, maximizing compiler optimization opportunities.
    - **`strip = true`**: Strips all debugging symbols and symbol tables from the final binary, preventing unnecessary
  metadata bloat.
