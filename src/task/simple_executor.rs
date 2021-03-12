use super::Task;
use alloc::collections::VecDeque;

pub struct SimpleExecutor {
  task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
  pub fn new() -> SimpleExecutor {
    SimpleExecutor {
      task_queue: VecDeque::new(),
    }
  }

  pub fn spawn(&mut self, task: Task) {
    self.task_queue.push_back(task)
  }
}

use core::task::{RawWaker, Waker};

fn dummy_waker() -> Waker {
  unsafe { Waker::from_raw(dummy_raw_waker()) } // unsafe because undefined behavior can occur if the programmer does not uphold the documented requirements of RawWaker
}

use core::task::RawWakerVTable;

fn dummy_raw_waker() -> RawWaker {
  fn noop(_: *const ()) {}
  fn clone(_: *const ()) -> RawWaker {
    dummy_raw_waker()
  }

  let vtable = &RawWakerVTable::new(clone, noop, noop, noop);
  RawWaker::new(0 as *const () /* null ptr */, vtable)
}

//////////////////////////////////////////////////////////

use core::task::{Context, Poll};

impl SimpleExecutor {
  pub fn run(&mut self) {
    while let Some(mut task) = self.task_queue.pop_front() {
      let waker = dummy_waker();
      let mut context = Context::from_waker(&waker);

      match task.poll(&mut context) {
        Poll::Ready(()) => {} // task done
        Poll::Pending => self.task_queue.push_back(task),
      }
    }
  }
}
