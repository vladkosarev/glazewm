use wm_common::CenterFocusedColumn;

use crate::traits::MIN_TILING_SIZE;

/// Allowed rounding error (in pixels) when checking whether a column fits
/// within the viewport.
const VISIBILITY_TOLERANCE: i32 = 1;

/// Horizontal strip of columns used by the scrolling layout.
///
/// Column positions are relative to the start of the strip. The viewport
/// is a window onto the strip, offset by a scroll offset.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollingStrip {
  /// Start position and width (in pixels) of each column.
  columns: Vec<(i32, i32)>,

  /// Width of the viewport (i.e. the workspace) in pixels.
  viewport_width: i32,
}

impl ScrollingStrip {
  /// Creates a strip from column widths given as fractions of the
  /// viewport width.
  ///
  /// Column edges are rounded from their cumulative position, such that
  /// columns whose sizes sum to 1 fill the viewport exactly.
  #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
  pub fn new(column_sizes: &[f32], viewport_width: i32, gap: i32) -> Self {
    let unit_width = (viewport_width + gap) as f32;
    let mut columns = Vec::with_capacity(column_sizes.len());
    let mut cumulative_size = 0.;

    for &size in column_sizes {
      let start = (cumulative_size * unit_width).round() as i32;
      cumulative_size += clamp_column_size(size);
      let end = (cumulative_size * unit_width).round() as i32 - gap;

      columns.push((start, (end - start).max(1)));
    }

    Self {
      columns,
      viewport_width,
    }
  }

  /// Start position and width (in pixels) of the column at the given
  /// index.
  pub fn column(&self, index: usize) -> Option<(i32, i32)> {
    self.columns.get(index).copied()
  }

  /// Total width of the strip in pixels.
  pub fn width(&self) -> i32 {
    self
      .columns
      .last()
      .map_or(0, |(start, width)| start + width)
  }

  /// Whether the column at the given index should be shown.
  ///
  /// Columns that fit entirely within the viewport are always shown.
  /// Columns that are cut off by an edge of the viewport are only shown
  /// if peeking is allowed on that edge.
  pub fn is_column_visible(
    &self,
    index: usize,
    scroll_offset: i32,
    can_peek_left: bool,
    can_peek_right: bool,
  ) -> bool {
    self.column(index).is_some_and(|(start, width)| {
      let viewport_end = scroll_offset + self.viewport_width;
      let is_cut_off_left = start < scroll_offset - VISIBILITY_TOLERANCE;
      let is_cut_off_right =
        start + width > viewport_end + VISIBILITY_TOLERANCE;

      self.is_column_in_view(index, scroll_offset)
        && (!is_cut_off_left || can_peek_left)
        && (!is_cut_off_right || can_peek_right)
    })
  }

  /// Whether any part of the column at the given index is within the
  /// viewport.
  pub fn is_column_in_view(
    &self,
    index: usize,
    scroll_offset: i32,
  ) -> bool {
    self.column(index).is_some_and(|(start, width)| {
      start < scroll_offset + self.viewport_width
        && start + width > scroll_offset
    })
  }

  /// Gets the scroll offset needed to show the focused column.
  ///
  /// `prev_focused` is the previously focused column, which is used by
  /// `CenterFocusedColumn::OnOverflow`.
  pub fn scroll_offset_for_focus(
    &self,
    focused: Option<usize>,
    prev_focused: Option<usize>,
    current_offset: i32,
    center_mode: &CenterFocusedColumn,
  ) -> i32 {
    let Some((start, width)) = focused.and_then(|i| self.column(i)) else {
      return self.clamp_offset(current_offset);
    };

    let centered_offset = self.offset_to_center(start, width);

    match center_mode {
      CenterFocusedColumn::Always => centered_offset,
      CenterFocusedColumn::Never => {
        self.scroll_into_view(start, width, current_offset)
      }
      CenterFocusedColumn::OnOverflow => {
        let fits_with_prev = prev_focused
          .and_then(|i| self.column(i))
          .is_none_or(|(prev_start, prev_width)| {
            let span_start = start.min(prev_start);
            let span_end = (start + width).max(prev_start + prev_width);
            span_end - span_start <= self.viewport_width
          });

        if fits_with_prev {
          self.scroll_into_view(start, width, current_offset)
        } else {
          self.clamp_offset(centered_offset)
        }
      }
    }
  }

  /// Gets the scroll offset that centers the column at the given index
  /// within the viewport.
  pub fn centered_offset(&self, index: usize) -> Option<i32> {
    self
      .column(index)
      .map(|(start, width)| self.offset_to_center(start, width))
  }

  /// Gets the offset that centers the given column span.
  fn offset_to_center(&self, start: i32, width: i32) -> i32 {
    start - (self.viewport_width - width) / 2
  }

  /// Gets the offset closest to `current_offset` that shows the given
  /// column span in its entirety.
  ///
  /// The offset is left unchanged if the column span is already in view.
  /// This preserves manual scrolling (e.g. via `center-column`), even if
  /// it leaves empty space at the ends of the strip.
  fn scroll_into_view(
    &self,
    start: i32,
    width: i32,
    current_offset: i32,
  ) -> i32 {
    if start < current_offset - VISIBILITY_TOLERANCE {
      self.clamp_offset(start)
    } else if start + width
      > current_offset + self.viewport_width + VISIBILITY_TOLERANCE
    {
      self.clamp_offset(start + width - self.viewport_width)
    } else {
      current_offset
    }
  }

  /// Clamps the offset such that no empty space is shown at either end of
  /// the strip.
  fn clamp_offset(&self, offset: i32) -> i32 {
    offset.clamp(0, (self.width() - self.viewport_width).max(0))
  }
}

/// Converts a column width in pixels to a column size (fraction of the
/// viewport).
///
/// Gaps are accounted for, such that e.g. two columns of 490px with a 20px
/// gap in a 1000px viewport are each of size `0.5`.
#[allow(clippy::cast_precision_loss)]
pub fn column_size_from_px(
  width: i32,
  viewport_width: i32,
  gap: i32,
) -> f32 {
  let total = (viewport_width + gap).max(1) as f32;
  clamp_column_size((width + gap) as f32 / total)
}

/// Clamps a column size to be within the minimum tiling size and the
/// full viewport width.
pub fn clamp_column_size(size: f32) -> f32 {
  size.clamp(MIN_TILING_SIZE, 1.0)
}

#[cfg(test)]
mod tests {
  use wm_common::CenterFocusedColumn;

  use super::{column_size_from_px, ScrollingStrip};

  const VIEWPORT: i32 = 1000;
  const GAP: i32 = 20;

  #[test]
  fn half_columns_fit_viewport_exactly() {
    let strip = ScrollingStrip::new(&[0.5, 0.5], VIEWPORT, GAP);

    assert_eq!(strip.column(0), Some((0, 490)));
    assert_eq!(strip.column(1), Some((510, 490)));
    assert_eq!(strip.width(), VIEWPORT);
  }

  #[test]
  fn third_columns_fill_viewport_exactly() {
    let strip =
      ScrollingStrip::new(&[1. / 3., 1. / 3., 1. / 3.], 1001, GAP);

    assert_eq!(strip.width(), 1001);
    assert!(strip.is_column_visible(2, 0, false, false));
  }

  #[test]
  fn column_width_is_capped_at_viewport() {
    let strip = ScrollingStrip::new(&[1.5, 1.0], VIEWPORT, GAP);

    assert_eq!(strip.column(0), Some((0, VIEWPORT)));
    assert_eq!(strip.column(1), Some((VIEWPORT + GAP, VIEWPORT)));
  }

  #[test]
  fn px_and_size_conversion_round_trips() {
    let size = column_size_from_px(490, VIEWPORT, GAP);
    let strip = ScrollingStrip::new(&[size], VIEWPORT, GAP);

    assert_eq!(strip.column(0), Some((0, 490)));
  }

  #[test]
  fn never_mode_scrolls_minimally() {
    let strip = ScrollingStrip::new(&[0.5, 0.5, 0.5, 0.5], VIEWPORT, GAP);
    let mode = CenterFocusedColumn::Never;

    // Focusing a visible column doesn't scroll.
    assert_eq!(strip.scroll_offset_for_focus(Some(1), None, 0, &mode), 0);

    // Focusing a column to the right aligns it with the right edge.
    assert_eq!(
      strip.scroll_offset_for_focus(Some(2), None, 0, &mode),
      510
    );

    // Focusing a column to the left aligns it with the left edge.
    assert_eq!(
      strip.scroll_offset_for_focus(Some(1), None, 1020, &mode),
      510
    );
  }

  #[test]
  fn always_mode_centers_focused_column() {
    let strip = ScrollingStrip::new(&[0.5, 0.5, 0.5], VIEWPORT, GAP);
    let offset = strip.scroll_offset_for_focus(
      Some(1),
      None,
      0,
      &CenterFocusedColumn::Always,
    );

    // Column 1 spans 510..1000, so its center (755) is at the viewport
    // center (offset + 500).
    assert_eq!(offset, 255);
  }

  #[test]
  fn on_overflow_mode_centers_only_when_prev_does_not_fit() {
    let strip =
      ScrollingStrip::new(&[0.5, 0.5, 0.5, 0.5, 0.5], VIEWPORT, GAP);
    let mode = CenterFocusedColumn::OnOverflow;

    // Adjacent columns fit together, so scroll minimally.
    assert_eq!(
      strip.scroll_offset_for_focus(Some(2), Some(1), 0, &mode),
      510
    );

    // Columns 0 and 3 don't fit together, so center column 3.
    assert_eq!(
      strip.scroll_offset_for_focus(Some(3), Some(0), 0, &mode),
      1275
    );
  }

  #[test]
  fn offset_is_clamped_after_strip_shrinks() {
    let strip = ScrollingStrip::new(&[0.5, 0.5], VIEWPORT, GAP);
    let offset = strip.scroll_offset_for_focus(
      Some(1),
      None,
      2000,
      &CenterFocusedColumn::Never,
    );

    assert_eq!(offset, 0);
  }

  #[test]
  fn manual_centering_is_preserved_while_focused_column_is_in_view() {
    let strip = ScrollingStrip::new(&[0.5, 0.5], VIEWPORT, GAP);
    let centered = strip.centered_offset(0).unwrap();

    // Centering the first column leaves empty space on the left.
    assert_eq!(centered, -255);

    assert_eq!(
      strip.scroll_offset_for_focus(
        Some(0),
        None,
        centered,
        &CenterFocusedColumn::Never,
      ),
      centered
    );
  }

  #[test]
  fn cut_off_columns_are_only_visible_if_peeking_is_allowed() {
    let strip = ScrollingStrip::new(&[0.5, 0.5, 0.5], VIEWPORT, GAP);

    assert!(strip.is_column_visible(0, 0, false, false));
    assert!(strip.is_column_visible(1, 0, false, false));

    // Column 2 starts beyond the viewport, so it can't peek.
    assert!(!strip.is_column_visible(2, 0, true, true));

    // Partially scrolled, so column 0 and 2 are cut off.
    assert!(!strip.is_column_visible(0, 255, false, true));
    assert!(strip.is_column_visible(0, 255, true, false));
    assert!(strip.is_column_visible(1, 255, false, false));
    assert!(!strip.is_column_visible(2, 255, true, false));
    assert!(strip.is_column_visible(2, 255, false, true));
  }
}
