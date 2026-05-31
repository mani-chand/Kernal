use x86_64::structures::idt::{ InterruptDescriptorTable, InterruptStackFrame };
use x86_64::instructions::port::Port;
use crate::println;
use crate::print;
use lazy_static::lazy_static;
use spin::Mutex;

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
    Timer = PIC_1_OFFSET,     // IRQ 0 is the timer
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
    _error_code: u64,
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
    // Read the scan code of the pressed key from Port 0x60
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Print the raw scan code to the screen
    print!("0x{:x} ", scancode);

    // Notify the PIC that we finished handling the interrupt
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
