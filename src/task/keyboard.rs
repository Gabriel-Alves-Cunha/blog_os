use crate::{print, println};
use conquer_once::spin::OnceCell; // Since the ArrayQueue::new performs a heap allocation, which is not possible at compile time (yet), we can't initialize the static variable directly. Instead, we use the OnceCell type of the conquer_once crate, which makes it possible to perform safe one-time initialization of static values. Instead of the OnceCell primitive, we could also use the lazy_static macro here. However, the OnceCell type has the advantage that we can ensure that the initialization does not happen in the interrupt handler, thus preventing that the interrupt handler performs a heap allocation.
use crossbeam_queue::ArrayQueue;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

// Called by the keyboard interrupt handler. Must not block or allocate
pub(crate) fn add_scancode(scancode: u8) {
  // Since this function should not be callable from our main.rs, we use the pub(crate) visibility to make it only available to our lib.rs.
  if let Ok(queue) = SCANCODE_QUEUE.try_get() {
    if let Err(_) = queue.push(scancode) {
      println!("WARNING: scancode queue full; dropping keyboard input!");
    } else {
      WAKER.wake();
    }
  } else {
    println!("WARNING: scancode queue uninitialied!");
  }
}

pub struct ScancodeStream {
  _private: (), // The purpose of the _private field is to prevent construction of the struct from outside of the module. This makes the new function the only way to construct the type.
}

impl ScancodeStream {
  pub fn new() -> Self {
    SCANCODE_QUEUE
      .try_init_once(|| ArrayQueue::new(100))
      .expect("ScancodeStream::new() should only be called once!");

    ScancodeStream { _private: () }
  }
}

use core::{
  pin::Pin,
  task::{Context, Poll},
};

impl Stream for ScancodeStream {
  type Item = u8;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
    let queue = SCANCODE_QUEUE
      .try_get()
      .expect("Scancode queue not initialized!");

    // fast path: avoid the performance overhead of registering a waker when the queue is not empty.
    if let Ok(scancode) = queue.pop() {
      return Poll::Ready(Some(scancode));
    }

    WAKER.register(&cx.waker());
    match queue.pop() {
      Ok(scancode) => {
        WAKER.take();
        Poll::Ready(Some(scancode))
      }
      Err(crossbeam_queue::PopError) => Poll::Pending,
    }
  }
}

use futures_util::{
  stream::{Stream, StreamExt},
  task::AtomicWaker,
};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

static WAKER: AtomicWaker = AtomicWaker::new();

pub async fn print_keypresses() {
  let mut scancodes = ScancodeStream::new();
  let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

  while let Some(scancode) = scancodes.next().await {
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
      if let Some(key) = keyboard.process_keyevent(key_event) {
        match key {
          DecodedKey::Unicode(char) => print!("{}", char),
          DecodedKey::RawKey(key) => print!("{:?}", key),
        }
      }
    }
  }
}
