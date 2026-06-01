use core::{ future::Future, pin::Pin };
use core::task::{ Context, Poll, Waker, RawWaker, RawWakerVTable };
use alloc::boxed::Box;
use alloc::collections::VecDeque;

/// A wrapper around a pinned, heap-allocated Future
pub struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            future: Box::pin(future),
        }
    }
}

/// The Task Scheduler / Executor
pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    /// Spawns a new task into the queue
    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task);
    }

    /// Runs all tasks in the queue until they are finished
    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);

            // Poll the future. If it returns Pending, put it back at the end of the queue.
            match task.future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {} // Task complete, discard it
                Poll::Pending => {
                    self.task_queue.push_back(task); // Re-queue task to run later
                }
            }
        }
    }
}

// --- Future Yielding (Cooperative pause) ---

pub struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            Poll::Pending // Tell the executor to pause this task and run another
        }
    }
}

/// Toggles a pause in the current async task
pub fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

// --- Dummy Waker Implementation ---
// Required by Rust's async system. Since our cooperative executor
// just spins through a queue, we don't need a real notification waker.

fn dummy_raw_waker() -> RawWaker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| dummy_raw_waker(), // clone
        |_| {}, // wake
        |_| {}, // wake_by_ref
        |_| {} // drop
    );
    RawWaker::new(core::ptr::null(), &VTABLE)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
