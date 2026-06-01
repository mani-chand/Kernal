# Arch-Rust OS Kernel

A minimal, freestanding `no_std` x86_64 operating system kernel written in Rust that boots into a graphical framebuffer environment inside QEMU using a modern BIOS bootloader.

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
                  │ (Graphical Framebuffer)│
                  └────────────────────────┘
```

---

## 1. Project Architecture

The project is structured as a **Cargo Workspace** consisting of two main components:

1. **[kernel](file:///D:/rust/kernal/os/kernel)**: The core freestanding kernel crate compiled for the bare-metal `x86_64-unknown-none` target.
   - Disables the standard library via `#![no_std]` and overrides the default program entry points using `#![no_main]`.
   - Implements CPU exception handling (breakpoint, double fault) and remaps the 8259 PIC to handle hardware interrupts.
   - Utilizes a custom [FrameBufferWriter](file:///D:/rust/kernal/os/kernel/src/main.rs#L126) to render text on a pixel-based graphical framebuffer.
2. **[runner](file:///D:/rust/kernal/os)** (Workspace Root): The host application defined in [src/main.rs](file:///D:/rust/kernal/os/src/main.rs) that orchestrates building, packaging, and emulation:
   - Automatically invokes Cargo with `-Z build-std=core,compiler_builtins` to compile standard library dependencies for bare metal.
   - Packages the compiled ELF executable into a bootable raw BIOS disk image (`bios.img`) using `bootloader::BiosBoot`.
   - Locates the local QEMU system emulator, configures library search paths (critical for Android SDK virtual tools), and boots the image.

---

## 2. Key OS Features

### A. Graphical Framebuffer Rendering

Unlike primitive kernels that write directly to the 80x25 character VGA text buffer at `0xb8000`, this kernel writes to a pixel-level graphical framebuffer supplied by `bootloader_api`.

- **Custom Font Engine**: Implements an 8x8 font bitmap representation for printable ASCII characters (from space ` ` to tilde `~`).
- **Font Scaling**: Draws characters using custom pixel scaling ($2\times$ scale) for high visibility.
- **Color Styling**: Displays text in a bright terminal green (`RGB: 0x00, 0xff, 0x00`) over a solid black background.
- **Custom Cursor**: Renders a dynamic blinkless white horizontal block cursor to indicate current input position.

### B. Interactive Command Shell

The kernel boots into an interactive shell with the prompt `arch-rust >`. The system supports entering commands, processing strings, and displaying outputs:

- **Command Buffer**: Accumulates keystrokes in a thread-safe global static [CMD_BUFFER](file:///D:/rust/kernal/os/kernel/src/main.rs#L388) (type [CommandBuf](file:///D:/rust/kernal/os/kernel/src/main.rs#L393)) up to a maximum length of 80 characters.
- **Command Interpreter**: When `Enter` is pressed, the shell processes commands in [interpret_command](file:///D:/rust/kernal/os/kernel/src/main.rs#L424). Supported commands:
  - `help`: Displays a list of available command shell tools.
  - `clear`: Empties the screen and resets the cursor to the top-left corner.
  - `about`: Prints OS version details and operating system architecture.
  - `echo <text>`: Repeats the provided text back to the screen.
  - `panic`: Intentionally triggers a CPU kernel panic to test recovery/halt handlers.

### C. Advanced Keyboard Driver & Line History

Hardware keyboard interrupts are captured via the legacy 8259 Programmable Interrupt Controller (PIC) mapping IRQ 1:

- **Scancode Interpretation**: Read from I/O Port `0x60` and decoded into standard characters using the `pc-keyboard` crate.
- **Smart Backspacing**: Intercepts `Backspace` input, popped from [CMD_BUFFER](file:///D:/rust/kernal/os/kernel/src/main.rs#L388), and triggers [erase_last_char](file:///D:/rust/kernal/os/kernel/src/main.rs#L165).
- **Line-Wrapping History**: Tracks end-of-line pixel positions in a circular structure (`line_end_positions` and `current_line_index`). This allows the backspace key to cleanly wrap *backwards* onto previous lines when deleting multi-line commands.
- **CPU Halting**: Employs the `hlt` assembly instruction inside the idle loop [kernel_main](file:///D:/rust/kernal/os/kernel/src/main.rs#L472) and panic handler, putting the CPU into a low-power state until the next hardware interrupt triggers.

### D. Interrupt & Exception Handling

A custom Interrupt Descriptor Table (IDT) is set up and loaded in [init_idt](file:///D:/rust/kernal/os/kernel/src/interrupts.rs#L58).

- **[breakpoint_handler](file:///D:/rust/kernal/os/kernel/src/interrupts.rs#L65)**: Handles CPU debug breakpoints (`int3` instruction) without crashing.
- **[double_fault_handler](file:///D:/rust/kernal/os/kernel/src/interrupts.rs#L70)**: Catches unhandled faults, preventing dangerous CPU triple faults (which trigger system resets) by putting the processor in a secure halt loop.
- **Remapped PIC**: Configures the chained [PICS](file:///D:/rust/kernal/os/kernel/src/interrupts.rs#L20) to offset IRQ signals to vector offsets `32` and `40` to avoid overlap conflicts with processor exceptions.

---

## 3. Binary Size & Compiler Optimizations

To run on bare-metal architectures, the kernel's binary footprint must be strictly controlled. Workspace compilation profiles are defined in the workspace root [Cargo.toml](file:///D:/rust/kernal/os/Cargo.toml) to reduce size:

- **`panic = "immediate-abort"`**: Enabled via standard `panic-immediate-abort` cargo-features. Removes panic string parsing, formatting infrastructure, and file location outputs, reducing binary overhead.
- **`opt-level = "z"`**: Directs the compiler to prioritize binary size optimization above execution speed.
- **`lto = true`**: Enables Link-Time Optimization (LTO) to optimize functions across crate boundaries, eliminating dead code paths.
- **`codegen-units = 1`**: Instructs the compiler to output code as a single optimization block, enabling aggressive inline optimization.
- **`strip = true`**: Removes debugging symbols and symbol tables from the final ELF output.

> [!NOTE]
> These optimizations reduce the final freestanding kernel ELF binary size from **~2.73 MB** (unoptimized debug build) down to **~3.03 KB** (optimized release build)—representing a **99.88%** total footprint savings.

---

## 4. Setup & Installation

Ensure you have the following packages and tools installed on your system.

### A. Rust Nightly GNU Toolchain

This workspace requires the Windows GNU Nightly toolchain:

```bash
rustup override set nightly-x86_64-pc-windows-gnu
```

### B. Required Toolchain Components

Install target platforms and source components:

```bash
# Core standard library source (needed to build core and compiler_builtins on bare-metal)
rustup component add rust-src

# LLVM utilities (used by packaging steps to create the raw bios disk image)
rustup component add llvm-tools-preview

# Bootloader packaging compilation target
rustup target add x86_64-unknown-uefi
```

### C. QEMU Emulator

A standalone QEMU emulator is required.

> [!WARNING]
> Do not use the virtualized QEMU binaries bundled within the Android SDK emulator. They contain external wrappers and custom Qt components that will lead to Windows DLL initialization errors (`0xc0000135`).

Install a clean, standalone version of QEMU using Chocolatey (from an elevated Administrator shell):

```powershell
choco install qemu -y
```

By default, this installs QEMU to `C:\Program Files\qemu\qemu-system-x86_64.exe`.

---

## 5. Configuration & Environment Variables

You can customize the QEMU binary path using an environment configuration file:

1. Copy the provided template [.env.example](file:///D:/rust/kernal/os/.env.example) to `.env`:

   ```bash
   copy .env.example .env
   ```

2. Edit `.env` to define your custom QEMU installation path:

   ```ini
   QEMU_PATH=C:\Program Files\qemu\qemu-system-x86_64.exe
   ```

The workspace [runner](file:///D:/rust/kernal/os/src/main.rs) loads this configuration dynamically and prioritizes `QEMU_PATH` before looking up default paths.

---

## 6. How to Build & Run

To compile the kernel, construct the bootable BIOS image, and launch the virtual machine, simply run:

```bash
cargo run
```

This triggers the host runner program which performs the following tasks:

1. Compiles the **[kernel](file:///D:/rust/kernal/os/kernel)** crate for the `x86_64-unknown-none` target.
2. Creates the BIOS boot disk image at `target/x86_64-unknown-none/release/bios.img`.
3. Locates QEMU, loads optional configuration paths, and fires up the machine:

   ```bash
   qemu-system-x86_64 -drive format=raw,file=target/x86_64-unknown-none/release/bios.img
   ```

A graphical window will launch displaying the `arch-rust >` interactive CLI command shell!

---

## 7. File Directory Map

- [Cargo.toml](file:///D:/rust/kernal/os/Cargo.toml) — Workspace configurations and compiler size optimizations.
- [src/main.rs](file:///D:/rust/kernal/os/src/main.rs) — Host runner tool that compiles the kernel, generates the boot image, and invokes QEMU.
- [kernel/Cargo.toml](file:///D:/rust/kernal/os/kernel/Cargo.toml) — Kernel crate configurations, settings, and low-level x86 dependencies.
- [kernel/src/main.rs](file:///D:/rust/kernal/os/kernel/src/main.rs) — Core entry point, graphical [FrameBufferWriter](file:///D:/rust/kernal/os/kernel/src/main.rs#L126), font engines, custom formatting macros (`print!`/`println!`), command parser, and CPU halt loop.
- [kernel/src/interrupts.rs](file:///D:/rust/kernal/os/kernel/src/interrupts.rs) — Interrupt Descriptor Table (IDT), CPU fault exception routines, and keyboard driver mapping IRQ 1.

  ## 8. Dynamic Console Colors

    Upgraded the text rendering engine to support multiple dynamic text colors by creating the `print_color!` and
  `println_color!` formatting macros. The console prompt renders in bright cyan, user typing is printed in white,
  error messages are highlighted in red, and the help system prints in yellow.

  ## 9. Programmable Interval Timer (PIT) & Sleep

  - Implemented a PIT driver in time.rs that generates periodic interrupts every 1ms (at 1000 Hz).
  - Implemented a thread-safe atomic counter `TICKS` updated on every timer interrupt.
  - Designed a low-power `sleep(ms)` function that halts the CPU (`hlt`) until the specified time duration has
  passed.

  ## 10. PC Speaker Sound Driver

    Designed a speaker driver in speaker.rs that writes to PIT Channel 2 (Port `0x42`) to set audio frequencies
  and toggles Port `0x61` to turn speaker sound on and off. Added a `beep` command to the shell that plays tones for
  precise millisecond durations using the timer
      ## 11. RAM Disk File System
  - Designed a standard TAR archive reader in ramdisk.rs that parses a compile-time embedded tar file (`ramdisk.
  tar`) without requiring heap allocation.
  - Implemented `ls` and `cat` commands in the shell.
  - Upgraded the `ls` command to parse UNIX-like options (e.g., `ls -l`, `ls -lh`, `ls -h`) to display detailed
  files and human-readable byte sizes (formatting sizes without using floating-point operations).

  ## 12. Heap Memory Management

  - Configured a dynamic memory pool of 100 KB using a static regional array buffer.
  - Integrated the `linked_list_allocator` crate and registered it as the global allocator (`#[global_allocator]`).
  - Enabled the standard `alloc` library crate compilation in the workspace runner build flags, unlocking full
  support for `Box`, `Vec`, `String`, and standard collections.

  ## 13. Async Cooperative Multitasking Scheduler

  - Implemented a custom asynchronous executor in task.rs using Rust's first-class `async/await` and `Future`
  abstractions.
  - Created a `SimpleExecutor` round-robin scheduler to queue and run multiple tasks cooperatively.
  - Designed a `yield_now` future to allow active threads to save CPU cycles and yield control back to the task
  scheduler.
