use std::time::Duration;

/// Interval between frames (~60 FPS).
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Clock for waiting on display refreshes.
pub(crate) struct FrameClock;

impl FrameClock {
  /// Creates a clock with a fixed frame interval.
  pub(crate) fn new() -> Self {
    Self
  }

  /// Blocks until the next frame.
  #[allow(clippy::unused_self)]
  pub(crate) fn wait_for_next_frame(&self) {
    std::thread::sleep(FRAME_INTERVAL);
  }
}
