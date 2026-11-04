use std::time::{Duration, Instant};

/// Animation of the scroll offset of the scrolling layout's viewport.
///
/// Uses an ease-out cubic curve, such that the animation starts fast and
/// decelerates towards the target offset.
#[derive(Clone, Debug)]
pub struct ScrollAnimation {
  /// Scroll offset at the start of the animation.
  from: i32,

  /// Scroll offset at the end of the animation.
  to: i32,

  start: Instant,
  duration: Duration,
}

impl ScrollAnimation {
  /// Creates an animation from one scroll offset to another, starting at
  /// the given time.
  pub fn new(
    from: i32,
    to: i32,
    start: Instant,
    duration: Duration,
  ) -> Self {
    Self {
      from,
      to,
      start,
      duration,
    }
  }

  /// Whether the animation has completed at the given time.
  pub fn is_finished_at(&self, now: Instant) -> bool {
    now.saturating_duration_since(self.start) >= self.duration
  }

  /// Gets the interpolated scroll offset at the given time.
  #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
  pub fn offset_at(&self, now: Instant) -> i32 {
    if self.is_finished_at(now) {
      return self.to;
    }

    let progress = now.saturating_duration_since(self.start).as_secs_f32()
      / self.duration.as_secs_f32();

    let eased_progress = 1. - (1. - progress).powi(3);

    self.from
      + ((self.to - self.from) as f32 * eased_progress).round() as i32
  }
}

#[cfg(test)]
mod tests {
  use std::time::{Duration, Instant};

  use super::ScrollAnimation;

  #[test]
  fn interpolates_with_ease_out() {
    let start = Instant::now();
    let animation =
      ScrollAnimation::new(0, 1000, start, Duration::from_millis(100));

    assert_eq!(animation.offset_at(start), 0);

    // Halfway through, ease-out cubic has covered 87.5% of the distance.
    assert_eq!(
      animation.offset_at(start + Duration::from_millis(50)),
      875
    );

    assert!(!animation.is_finished_at(start + Duration::from_millis(99)));
  }

  #[test]
  fn ends_at_target() {
    let start = Instant::now();
    let animation =
      ScrollAnimation::new(500, -200, start, Duration::from_millis(100));

    let end = start + Duration::from_millis(100);
    assert!(animation.is_finished_at(end));
    assert_eq!(animation.offset_at(end), -200);
    assert_eq!(animation.offset_at(end + Duration::from_secs(1)), -200);
  }
}
