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
  pub fn new(column_sizes: &[f32], viewport_width: i32, gap: i32) -> Self {
    let mut columns = Vec::with_capacity(column_sizes.len());
    let mut next_start = 0;

    for &size in column_sizes {
      let width = column_width_px(size, viewport_width, gap);
      columns.push((next_start, width));
      next_start += width + gap;
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

  /// Whether the column at the given index fits entirely within the
  /// viewport.
  pub fn is_column_visible(
    &self,
    index: usize,
    scroll_offset: i32,
  ) -> bool {
    self.column(index).is_some_and(|(start, width)| {
      start >= scroll_offset - VISIBILITY_TOLERANCE
        && start + width
          <= scroll_offset + self.viewport_width + VISIBILITY_TOLERANCE
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
      CenterFocusedColumn::Never => self
        .clamp_offset(self.minimal_offset(start, width, current_offset)),
      CenterFocusedColumn::OnOverflow => {
        let fits_with_prev = prev_focused
          .and_then(|i| self.column(i))
          .is_none_or(|(prev_start, prev_width)| {
            let span_start = start.min(prev_start);
            let span_end = (start + width).max(prev_start + prev_width);
            span_end - span_start <= self.viewport_width
          });

        if fits_with_prev {
          self.clamp_offset(self.minimal_offset(
            start,
            width,
            current_offset,
          ))
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
  /// column span.
  fn minimal_offset(
    &self,
    start: i32,
    width: i32,
    current_offset: i32,
  ) -> i32 {
    if start < current_offset {
      start
    } else if start + width > current_offset + self.viewport_width {
      start + width - self.viewport_width
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

/// Converts a column size (fraction of the viewport) to a width in pixels.
///
/// Gaps are accounted for, such that e.g. two columns of size `0.5` fit
/// exactly within the viewport.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn column_width_px(size: f32, viewport_width: i32, gap: i32) -> i32 {
  let size = clamp_column_size(size);
  let width = (size * (viewport_width + gap) as f32).round() as i32 - gap;
  width.clamp(1, viewport_width.max(1))
}

/// Converts a column width in pixels to a column size (fraction of the
/// viewport). Inverse of `column_width_px`.
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

  use super::{column_size_from_px, column_width_px, ScrollingStrip};

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
  fn column_width_is_capped_at_viewport() {
    assert_eq!(column_width_px(1.5, VIEWPORT, GAP), VIEWPORT);
    assert_eq!(column_width_px(1.0, VIEWPORT, GAP), VIEWPORT);
  }

  #[test]
  fn px_and_size_conversion_round_trips() {
    let size = column_size_from_px(490, VIEWPORT, GAP);
    assert_eq!(column_width_px(size, VIEWPORT, GAP), 490);
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
  fn only_fully_contained_columns_are_visible() {
    let strip = ScrollingStrip::new(&[0.5, 0.5, 0.5], VIEWPORT, GAP);

    assert!(strip.is_column_visible(0, 0));
    assert!(strip.is_column_visible(1, 0));
    assert!(!strip.is_column_visible(2, 0));

    // Partially scrolled, so column 0 is cut off.
    assert!(!strip.is_column_visible(0, 255));
    assert!(strip.is_column_visible(1, 255));
  }
}
