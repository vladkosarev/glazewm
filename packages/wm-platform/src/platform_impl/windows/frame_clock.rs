use std::time::Duration;

use windows::Win32::Graphics::Dxgi::{
  CreateDXGIFactory1, IDXGIFactory1, IDXGIOutput,
};

/// Interval to wait between frames if waiting for the vertical blank is
/// unavailable (~60 FPS).
const FALLBACK_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Clock for waiting on display refreshes.
pub(crate) struct FrameClock {
  /// Primary display output, if available.
  output: Option<IDXGIOutput>,
}

impl FrameClock {
  /// Creates a clock that waits on the primary display output.
  pub(crate) fn new() -> Self {
    let output = primary_output()
      .inspect_err(|err| {
        tracing::warn!(
          "Failed to get display output. Using fixed frame interval: {}",
          err
        );
      })
      .ok();

    Self { output }
  }

  /// Blocks until the next vertical blank of the primary display output.
  pub(crate) fn wait_for_next_frame(&self) {
    let has_waited = self
      .output
      .as_ref()
      // SAFETY: `output` is a valid DXGI output, and the call blocks the
      // current thread until the next vertical blank.
      .is_some_and(|output| unsafe { output.WaitForVBlank() }.is_ok());

    if !has_waited {
      std::thread::sleep(FALLBACK_FRAME_INTERVAL);
    }
  }
}

/// Gets the DXGI output of the primary display.
fn primary_output() -> crate::Result<IDXGIOutput> {
  // SAFETY: Creates a new DXGI factory. Adapter 0 and its output 0 are the
  // primary display adapter and output.
  unsafe {
    let factory = CreateDXGIFactory1::<IDXGIFactory1>()?;
    let adapter = factory.EnumAdapters1(0)?;
    Ok(adapter.EnumOutputs(0)?)
  }
}
