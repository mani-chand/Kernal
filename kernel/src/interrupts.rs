use x86_64::structures::idt::{ InterruptDescriptorTable, InterruptStackFrame };
use x86_64::instructions::port::Port;
use crate::println;
use crate::print;
use lazy_static::lazy_static;
use spin::Mutex;
use pc_keyboard::{ layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1 };

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
        Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::MapLettersToUnicode)
    );
}

// Remap PIC interrupts to start at vector 32 (IRQ 0-7) and 40 (IRQ 8-15)
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = 40;

// Set up the Programmable Interrupt Controller (PIC)
pub static PICS: Mutex<pic8259::ChainedPics> = Mutex::new(unsafe {
    pic8259::ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
});
// Hardware interrupts enumeration
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET, // IRQ 0 is the timer
    Keyboard = PIC_1_OFFSET + 1, // IRQ 1 is the keyboard
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// We use lazy_static because the IDT must persist in memory
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        // Register CPU Exception Handlers
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        // Register Hardware Interrupt Handlers
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

/// Initializes the IDT and loads it into the CPU
pub fn init_idt() {
    IDT.load();
}

// --- CPU Exception Handlers ---

/// Breakpoint handler: triggered when the CPU executes an `int3` instruction
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

/// Double Fault handler: triggered when the CPU fails to call an exception handler
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

// --- Hardware Interrupt Handlers ---

/// Timer handler: called on every clock tick (IRQ 0)
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // For now, we don't print on every tick (it would spam the screen),
    // but we MUST notify the PIC that the interrupt is handled.
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

/// Keyboard handler: called whenever a key is pressed or released (IRQ 1)
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                // If it's a newline (Enter), process the current command buffer
                DecodedKey::Unicode('\n') => {
                    println!(); // Print a newline to screen

                    let mut cmd_lock = crate::CMD_BUFFER.lock();
                    // Convert the command bytes to a string slice
                    if let Ok(cmd_str) = core::str::from_utf8(&cmd_lock.buf[..cmd_lock.len]) {
                        crate::interpret_command(cmd_str);
                    }
                    cmd_lock.clear(); // Clear the command buffer
                    crate::print_prompt(); // Draw a new prompt line
                }

                // If it's Backspace, remove from command buffer and screen
                DecodedKey::Unicode('\u{8}') => {
                    let mut cmd_lock = crate::CMD_BUFFER.lock();
                    if cmd_lock.pop() {
                        crate::erase_char();
                    }
                }

                // If it's standard printable character, push to buffer and print it
                DecodedKey::Unicode(character) => {
                    // Check if character is printable ASCII (codes 32 to 126)
                    let ascii_val = character as u8;
                    if (32..=126).contains(&ascii_val) {
                        let mut cmd_lock = crate::CMD_BUFFER.lock();
                        if cmd_lock.push(ascii_val) {
                            print!("{}", character);
                        }
                    }
                }

                // Catch raw Backspace key just in case
                DecodedKey::RawKey(pc_keyboard::KeyCode::Backspace) => {
                    let mut cmd_lock = crate::CMD_BUFFER.lock();
                    if cmd_lock.pop() {
                        crate::erase_char();
                    }
                }

                // Ignore other keys
                DecodedKey::RawKey(_raw_key) => {}
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
