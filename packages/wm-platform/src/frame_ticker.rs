use std::{
  sync::{Arc, Condvar, Mutex},
  thread::JoinHandle,
};

use tokio::sync::Notify;

use crate::platform_impl;

/// Emits ticks in sync with the display's refresh rate while enabled.
///
/// Intended for pacing animations. Ticks are coalesced, such that at most
/// one tick is pending if the receiver falls behind.
///
/// # Platform-specific
///
/// - **Windows**: Waits for the vertical blank of the primary display
///   output. Falls back to a fixed interval if this is unavailable.
/// - **macOS**: Uses a fixed interval.
pub struct FrameTicker {
  tick: Arc<Notify>,
  control: Arc<(Mutex<TickerState>, Condvar)>,
  thread: Option<JoinHandle<()>>,
}

#[derive(Debug, Default)]
struct TickerState {
  is_enabled: bool,
  is_terminated: bool,
}

impl FrameTicker {
  /// Creates a new [`FrameTicker`], which is initially disabled.
  pub fn new() -> crate::Result<Self> {
    let tick = Arc::new(Notify::new());
    let control =
      Arc::new((Mutex::new(TickerState::default()), Condvar::new()));

    let thread = std::thread::Builder::new()
      .name("frame-ticker".to_string())
      .spawn({
        let tick = tick.clone();
        let control = control.clone();
        move || run_ticker(&tick, &control)
      })?;

    Ok(Self {
      tick,
      control,
      thread: Some(thread),
    })
  }

  /// Enables or disables ticks. The ticker thread is idle while disabled.
  pub fn set_enabled(&self, is_enabled: bool) {
    let (state, condvar) = &*self.control;

    if let Ok(mut state) = state.lock() {
      if state.is_enabled != is_enabled {
        state.is_enabled = is_enabled;
        condvar.notify_one();
      }
    }
  }

  /// Returns on the next tick.
  pub async fn next_tick(&self) {
    self.tick.notified().await;
  }
}

impl Drop for FrameTicker {
  fn drop(&mut self) {
    let (state, condvar) = &*self.control;

    if let Ok(mut state) = state.lock() {
      state.is_terminated = true;
      condvar.notify_one();
    }

    if let Some(thread) = self.thread.take() {
      let _ = thread.join();
    }
  }
}

/// Runs the ticker loop until terminated.
fn run_ticker(tick: &Notify, control: &(Mutex<TickerState>, Condvar)) {
  let clock = platform_impl::FrameClock::new();
  let (state, condvar) = control;

  loop {
    // Wait until enabled or terminated.
    {
      let Ok(mut state) = state.lock() else {
        return;
      };

      while !state.is_enabled && !state.is_terminated {
        let Ok(next_state) = condvar.wait(state) else {
          return;
        };

        state = next_state;
      }

      if state.is_terminated {
        return;
      }
    }

    clock.wait_for_next_frame();
    tick.notify_one();
  }
}
