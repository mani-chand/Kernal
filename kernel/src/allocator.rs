use linked_list_allocator::LockedHeap;

// Register the global allocator for the entire Rust compiler
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Set aside a static 100 KB region of memory for our heap pool
const HEAP_SIZE: usize = 100 * 1024;
static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// Initialize the global allocator with our static heap memory
pub fn init_heap() {
    unsafe {
        ALLOCATOR.lock().init(HEAP_SPACE.as_ptr() as *mut u8, HEAP_SIZE);
    }
}
