use core::sync::atomic::{ AtomicU64, Ordering };
use x86_64::instructions::port::Port;

// Atomic global tick counter (safe to access across interrupts)
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Configure the PIT to trigger interrupts every 1 millisecond (1000 Hz)
pub fn init_pit() {
    // PIT base frequency is 1,193,182 Hz. Divisor = Base / Target Frequency.
    let divisor = 1193182 / 1000;

    let mut command_port = Port::new(0x43);
    let mut data_port = Port::new(0x40);

    unsafe {
        command_port.write(0x36_u8); // Channel 0, lobyte/hibyte, square wave mode
        data_port.write((divisor & 0xff) as u8);
        data_port.write(((divisor >> 8) & 0xff) as u8);
    }
}

/// Increments the system ticks (called by the timer interrupt)
pub fn increment_ticks() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Sleeps for a specified number of milliseconds
pub fn sleep(ms: u64) {
    let target = TICKS.load(Ordering::Relaxed) + ms;

    // Put the CPU to sleep until the timer interrupt wakes it up and tick reaches target
    while TICKS.load(Ordering::Relaxed) < target {
        x86_64::instructions::hlt();
    }
}
