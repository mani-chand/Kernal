use x86_64::instructions::port::Port;

/// Play a sound with a given frequency (Hz)
pub fn play_sound(frequency: u32) {
    if frequency == 0 {
        return;
    }

    // The PIT base frequency is 1,193,180 Hz. Divisor = Base Freq / Target Freq.
    let divisor = 1193180 / frequency;

    let mut command_port = Port::new(0x43);
    let mut data_port = Port::new(0x42);
    let mut speaker_port = Port::new(0x61);

    unsafe {
        // Set PIT Channel 2 to square wave mode (0xb6)
        command_port.write(0xb6_u8);

        // Write the frequency divisor (low byte, then high byte)
        data_port.write((divisor & 0xff) as u8);
        data_port.write(((divisor >> 8) & 0xff) as u8);

        // Turn the speaker gating on by setting bits 0 and 1 of Port 0x61
        let val: u8 = speaker_port.read();
        if (val & 3) != 3 {
            speaker_port.write(val | 3);
        }
    }
}

/// Turn off the speaker (stop sound)
pub fn stop_sound() {
    let mut speaker_port = Port::new(0x61);
    unsafe {
        let val: u8 = speaker_port.read();
        speaker_port.write(val & 0xfc); // Clear bits 0 and 1 to turn sound off
    }
}

/// Beep for a specified duration in milliseconds
pub fn beep(frequency: u32, duration_ms: u64) {
    play_sound(frequency);
    crate::time::sleep(duration_ms); // Use the new real system timer sleep
    stop_sound();
}
