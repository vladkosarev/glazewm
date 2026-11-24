use std::{
  cell::{Ref, RefCell, RefMut},
  collections::VecDeque,
  rc::Rc,
  time::{Duration, Instant},
};

use anyhow::Context;
use uuid::Uuid;
use wm_common::{
  ContainerDto, GapsConfig, LayoutConfig, LayoutType, TilingDirection,
  WorkspaceConfig, WorkspaceDto,
};
use wm_platform::{LengthUnit, LengthValue, Rect, RectDelta};

use crate::{
  impl_common_getters, impl_container_debug,
  impl_tiling_direction_getters,
  models::{
    clamp_column_size, column_size_from_px, Container, DirectionContainer,
    ScrollAnimation, ScrollingStrip, TilingContainer, WindowContainer,
  },
  traits::{
    CommonGetters, PositionGetters, TilingDirectionGetters,
    TilingSizeGetters,
  },
};

#[derive(Clone)]
pub struct Workspace(Rc<RefCell<WorkspaceInner>>);

#[derive(Debug)]
struct WorkspaceInner {
  id: Uuid,
  parent: Option<Container>,
  children: VecDeque<Container>,
  child_focus_order: VecDeque<Uuid>,
  config: WorkspaceConfig,
  gaps_config: GapsConfig,
  layout_config: LayoutConfig,
  tiling_direction: TilingDirection,

  /// Horizontal scroll offset (in pixels) of the viewport when using the
  /// scrolling layout. This is the target offset when animating.
  scroll_offset: i32,

  /// Scroll offset that was last committed to be displayed (see
  /// `Workspace::commit_scroll_offset`).
  committed_scroll_offset: i32,

  /// In-progress animation of the scroll offset, along with the current
  /// (i.e. displayed) offset of the animation.
  scroll_animation: Option<(ScrollAnimation, i32)>,

  /// Size of newly tiled columns in the scrolling layout, as set by
  /// `Workspace::set_columns_per_screen`. Overrides the configured
  /// default column width.
  column_size_override: Option<f32>,

  /// IDs of the columns as of the last `Workspace::sync_scroll_offset`.
  synced_column_ids: Vec<Uuid>,

  /// ID of the focused column as of the last
  /// `Workspace::sync_scroll_offset`.
  synced_focused_column_id: Option<Uuid>,

  /// Scroll offset to restore if the most recently opened column is
  /// closed (see `ScrollRestore`).
  scroll_restore: Option<ScrollRestore>,
}

/// Scroll offset to restore when a newly opened column is removed.
///
/// A column that opens to the right of the focused column and takes focus
/// might scroll the previously focused column (e.g. a centered column) to
/// the edge of the viewport. If the new column is then removed without
/// focus moving to another column in-between, the previously focused
/// column is shown as it was before.
#[derive(Clone, Debug)]
struct ScrollRestore {
  /// ID of the newly opened column.
  column_id: Uuid,

  /// ID of the column that was focused before the new column opened.
  prev_column_id: Uuid,

  /// Start position (in pixels) of the previously focused column within
  /// the scrolling strip.
  prev_column_start: i32,

  /// Scroll offset before the new column opened.
  scroll_offset: i32,
}

impl Workspace {
  pub fn new(
    config: WorkspaceConfig,
    gaps_config: GapsConfig,
    layout_config: LayoutConfig,
    tiling_direction: TilingDirection,
  ) -> Self {
    // The scrolling layout arranges columns horizontally.
    let tiling_direction = match layout_config.layout_type {
      LayoutType::Scrolling => TilingDirection::Horizontal,
      LayoutType::Tiling => tiling_direction,
    };

    let workspace = WorkspaceInner {
      id: Uuid::new_v4(),
      parent: None,
      children: VecDeque::new(),
      child_focus_order: VecDeque::new(),
      config,
      gaps_config,
      layout_config,
      tiling_direction,
      scroll_offset: 0,
      committed_scroll_offset: 0,
      scroll_animation: None,
      column_size_override: None,
      synced_column_ids: Vec::new(),
      synced_focused_column_id: None,
      scroll_restore: None,
    };

    Self(Rc::new(RefCell::new(workspace)))
  }

  /// Underlying config for the workspace.
  pub fn config(&self) -> WorkspaceConfig {
    self.0.borrow().config.clone()
  }

  /// Update the underlying config for the workspace.
  pub fn set_config(&self, config: WorkspaceConfig) {
    self.0.borrow_mut().config = config;
  }

  /// Whether the workspace is currently displayed by the parent monitor.
  pub fn is_displayed(&self) -> bool {
    self
      .monitor()
      .and_then(|monitor| monitor.displayed_workspace())
      .is_some_and(|workspace| workspace.id() == self.id())
  }

  pub fn set_gaps_config(&self, gaps_config: GapsConfig) {
    self.0.borrow_mut().gaps_config = gaps_config;
  }

  /// Layout config for the workspace.
  pub fn layout_config(&self) -> LayoutConfig {
    self.0.borrow().layout_config.clone()
  }

  /// Update the layout config for the workspace.
  ///
  /// Does not rearrange the workspace's children. Use `apply_layout` to
  /// switch between layouts.
  pub fn set_layout_config(&self, layout_config: LayoutConfig) {
    self.0.borrow_mut().layout_config = layout_config;
  }

  /// Whether the workspace uses the scrolling layout.
  pub fn is_scrolling(&self) -> bool {
    self.0.borrow().layout_config.layout_type == LayoutType::Scrolling
  }

  /// Horizontal scroll offset (in pixels) of the scrolling layout's
  /// viewport.
  ///
  /// While animating, this is the target offset of the animation.
  pub fn scroll_offset(&self) -> i32 {
    self.0.borrow().scroll_offset
  }

  /// Scroll offset that windows are currently displayed at.
  ///
  /// Lags behind `scroll_offset` until the scroll offset is committed via
  /// `commit_scroll_offset`, and while a scroll animation is in progress.
  pub fn displayed_scroll_offset(&self) -> i32 {
    let inner = self.0.borrow();

    inner
      .scroll_animation
      .as_ref()
      .map_or(inner.committed_scroll_offset, |(_, offset)| *offset)
  }

  /// Commits the scroll offset to be displayed.
  ///
  /// If an animation is given as a start time and duration, the change is
  /// animated from the currently displayed scroll offset (replacing any
  /// in-progress animation). Otherwise, the change is applied immediately.
  ///
  /// Returns whether the scroll offset changed since the last commit.
  pub fn commit_scroll_offset(
    &self,
    animation: Option<(Instant, Duration)>,
  ) -> bool {
    let from = self.displayed_scroll_offset();
    let mut inner = self.0.borrow_mut();
    let to = inner.scroll_offset;

    if to == inner.committed_scroll_offset {
      return false;
    }

    inner.committed_scroll_offset = to;
    inner.scroll_animation =
      animation.filter(|_| from != to).map(|(start, duration)| {
        (ScrollAnimation::new(from, to, start, duration), from)
      });

    true
  }

  /// Whether the scroll offset is currently being animated.
  pub fn has_scroll_animation(&self) -> bool {
    self.0.borrow().scroll_animation.is_some()
  }

  /// Advances the scroll animation to the given time.
  ///
  /// Returns `false` once the animation has finished (or if there is no
  /// animation), in which case windows are displayed at the committed
  /// scroll offset.
  pub fn step_scroll_animation(&self, now: Instant) -> bool {
    let mut inner = self.0.borrow_mut();

    let Some((animation, displayed_offset)) = &mut inner.scroll_animation
    else {
      return false;
    };

    if animation.is_finished_at(now) {
      inner.scroll_animation = None;
      return false;
    }

    *displayed_offset = animation.offset_at(now);
    true
  }

  /// Stops any in-progress scroll animation, such that windows are
  /// displayed at the committed scroll offset.
  pub fn stop_scroll_animation(&self) {
    self.0.borrow_mut().scroll_animation = None;
  }

  /// Gap (in pixels) between adjacent columns.
  fn horizontal_inner_gap(&self) -> anyhow::Result<i32> {
    let monitor = self.monitor().context("No monitor.")?;
    let gaps_config = &self.0.borrow().gaps_config;

    let scale_factor = if gaps_config.scale_with_dpi {
      monitor.native_properties().scale_factor
    } else {
      1.
    };

    // Matches the horizontal gap from `TilingSizeGetters::inner_gaps`.
    Ok(gaps_config.inner_gap.to_px(
      monitor.native_properties().bounds.height(),
      Some(scale_factor),
    ))
  }

  /// Gets the strip of columns for the scrolling layout.
  pub fn scrolling_strip(&self) -> anyhow::Result<ScrollingStrip> {
    let column_sizes = self
      .tiling_children()
      .map(|child| child.tiling_size())
      .collect::<Vec<_>>();

    Ok(ScrollingStrip::new(
      &column_sizes,
      self.to_rect()?.width(),
      self.horizontal_inner_gap()?,
    ))
  }

  /// Index of a column (i.e. tiling child) within the scrolling strip.
  fn column_index(&self, column_id: Uuid) -> Option<usize> {
    self
      .tiling_children()
      .position(|child| child.id() == column_id)
  }

  /// Gets the rect of a column in the scrolling layout, taking into
  /// account the displayed scroll offset.
  ///
  /// The rect can be outside of the workspace bounds if the column is
  /// scrolled out of view.
  pub fn scrolling_column_rect(
    &self,
    column_id: Uuid,
  ) -> anyhow::Result<Rect> {
    let workspace_rect = self.to_rect()?;

    let (start, width) = self
      .column_index(column_id)
      .and_then(|index| self.scrolling_strip().ok()?.column(index))
      .context("Container is not a column of the workspace.")?;

    Ok(Rect::from_xy(
      workspace_rect.x() + start - self.displayed_scroll_offset(),
      workspace_rect.y(),
      width,
      workspace_rect.height(),
    ))
  }

  /// Whether a column of the scrolling layout should be shown.
  ///
  /// Columns cut off by an edge of the viewport are only shown if peeking
  /// is allowed on that edge (see `ScrollingStrip::is_column_visible`).
  pub fn is_column_visible(
    &self,
    column_id: Uuid,
    can_peek_left: bool,
    can_peek_right: bool,
  ) -> bool {
    self.column_index(column_id).is_some_and(|index| {
      self.scrolling_strip().is_ok_and(|strip| {
        strip.is_column_visible(
          index,
          self.displayed_scroll_offset(),
          can_peek_left,
          can_peek_right,
        )
      })
    })
  }

  /// Updates the scroll offset of the scrolling layout to show the
  /// focused column.
  ///
  /// Returns whether the scroll offset changed.
  pub fn sync_scroll_offset(&self) -> anyhow::Result<bool> {
    let strip = self.scrolling_strip()?;

    let column_ids = self
      .tiling_children()
      .map(|child| child.id())
      .collect::<Vec<_>>();

    // Get indices of the most and second-most recently focused columns.
    let mut focus_order = self
      .tiling_children()
      .enumerate()
      .map(|(index, child)| (child.focus_index(), index))
      .collect::<Vec<_>>();

    focus_order.sort_unstable();
    let mut focused_columns = focus_order.into_iter().map(|(_, i)| i);
    let focused_index = focused_columns.next();
    let focused_id = focused_index.map(|index| column_ids[index]);

    let prev_scroll_offset = self.scroll_offset();
    let restored_offset =
      self.take_restored_scroll_offset(&strip, &column_ids, focused_id);

    let scroll_offset = strip.scroll_offset_for_focus(
      focused_index,
      focused_columns.next(),
      restored_offset.unwrap_or(prev_scroll_offset),
      &self.0.borrow().layout_config.center_focused_column,
    );

    self.track_new_column(&strip, &column_ids, focused_index);

    let mut inner = self.0.borrow_mut();
    inner.scroll_offset = scroll_offset;
    inner.synced_column_ids = column_ids;
    inner.synced_focused_column_id = focused_id;

    Ok(scroll_offset != prev_scroll_offset)
  }

  /// Gets the scroll offset to restore if the most recently opened column
  /// was removed and focus returned to the previously focused column.
  ///
  /// Discards the pending restore once it no longer applies (i.e. when
  /// focus moves away from the new column).
  fn take_restored_scroll_offset(
    &self,
    strip: &ScrollingStrip,
    column_ids: &[Uuid],
    focused_id: Option<Uuid>,
  ) -> Option<i32> {
    let restore = self.0.borrow_mut().scroll_restore.take()?;

    if column_ids.contains(&restore.column_id) {
      // Keep the restore while the new column remains focused.
      if focused_id == Some(restore.column_id) {
        self.0.borrow_mut().scroll_restore = Some(restore);
      }

      return None;
    }

    if focused_id != Some(restore.prev_column_id) {
      return None;
    }

    // Account for columns to the left having been resized or removed.
    let prev_index = column_ids
      .iter()
      .position(|id| *id == restore.prev_column_id)?;

    let (prev_column_start, _) = strip.column(prev_index)?;

    Some(
      restore.scroll_offset + prev_column_start
        - restore.prev_column_start,
    )
  }

  /// Sets up a `ScrollRestore` if a new column was opened to the right of
  /// the previously focused column and took focus.
  fn track_new_column(
    &self,
    strip: &ScrollingStrip,
    column_ids: &[Uuid],
    focused_index: Option<usize>,
  ) {
    let mut inner = self.0.borrow_mut();

    let Some(focused_index) = focused_index else {
      return;
    };

    let focused_id = column_ids[focused_index];

    if inner.synced_column_ids.contains(&focused_id) {
      return;
    }

    let prev_index = focused_index.checked_sub(1);

    let restore = prev_index.and_then(|prev_index| {
      let prev_column_id = column_ids[prev_index];

      if inner.synced_focused_column_id != Some(prev_column_id) {
        return None;
      }

      let (prev_column_start, _) = strip.column(prev_index)?;

      Some(ScrollRestore {
        column_id: focused_id,
        prev_column_id,
        prev_column_start,
        scroll_offset: inner.scroll_offset,
      })
    });

    if restore.is_some() {
      inner.scroll_restore = restore;
    }
  }

  /// Scrolls the viewport of the scrolling layout to center the given
  /// column.
  ///
  /// The offset is subsequently clamped by `sync_scroll_offset` unless the
  /// layout is configured to always center the focused column.
  pub fn center_column(&self, column_id: Uuid) -> anyhow::Result<()> {
    let centered_offset = self
      .column_index(column_id)
      .and_then(|index| {
        self.scrolling_strip().ok()?.centered_offset(index)
      })
      .context("Container is not a column of the workspace.")?;

    self.0.borrow_mut().scroll_offset = centered_offset;

    Ok(())
  }

  /// Indices of the columns of the scrolling layout that are (at least
  /// partially) within the viewport.
  fn column_indices_in_view(&self) -> anyhow::Result<Vec<usize>> {
    let strip = self.scrolling_strip()?;
    let scroll_offset = self.scroll_offset();

    Ok(
      (0..self.tiling_children().count())
        .filter(|&index| strip.is_column_in_view(index, scroll_offset))
        .collect(),
    )
  }

  /// Evenly sizes the columns of the scrolling layout that are (at least
  /// partially) within the viewport, such that they fill the viewport.
  ///
  /// Columns outside of the viewport keep their size.
  pub fn balance_columns_in_view(&self) -> anyhow::Result<()> {
    let indices_in_view = self.column_indices_in_view()?;

    let Some(&first_index) = indices_in_view.first() else {
      return Ok(());
    };

    #[allow(clippy::cast_precision_loss)]
    let column_size = 1.0 / indices_in_view.len() as f32;

    self.resize_columns_and_align(
      |index| indices_in_view.contains(&index),
      column_size,
      first_index,
    )
  }

  /// Sizes all columns of the scrolling layout such that the given number
  /// of columns fill the viewport.
  ///
  /// The first column that is currently within the viewport is aligned
  /// with the left edge of the viewport.
  pub fn set_columns_per_screen(&self, count: u32) -> anyhow::Result<()> {
    let align_index =
      self.column_indices_in_view()?.first().copied().unwrap_or(0);

    #[allow(clippy::cast_precision_loss)]
    let column_size = clamp_column_size(1.0 / count.max(1) as f32);

    // New columns are opened with the same size.
    self.0.borrow_mut().column_size_override = Some(column_size);

    self.resize_columns_and_align(|_| true, column_size, align_index)
  }

  /// Sets the size of the columns whose index matches the predicate, and
  /// then scrolls the viewport such that the column at `align_index` is at
  /// its left edge.
  fn resize_columns_and_align(
    &self,
    should_resize: impl Fn(usize) -> bool,
    column_size: f32,
    align_index: usize,
  ) -> anyhow::Result<()> {
    for (_, column) in self
      .tiling_children()
      .enumerate()
      .filter(|(index, _)| should_resize(*index))
    {
      column.set_tiling_size(clamp_column_size(column_size));
    }

    let strip = self.scrolling_strip()?;

    if let Some((start, _)) = strip.column(align_index) {
      self.0.borrow_mut().scroll_offset = strip.clamp_offset(start);
    }

    Ok(())
  }

  /// Resets the scroll offset of the scrolling layout's viewport.
  pub fn reset_scroll_offset(&self) {
    self.0.borrow_mut().scroll_offset = 0;
  }

  /// Converts a column width to a column size (fraction of the workspace
  /// width) for the scrolling layout.
  pub fn column_size(&self, width: &LengthValue) -> anyhow::Result<f32> {
    match width.unit {
      LengthUnit::Percentage => Ok(width.amount),
      LengthUnit::Pixel => {
        let monitor = self.monitor().context("No monitor.")?;

        let scale_factor = if self.0.borrow().gaps_config.scale_with_dpi {
          monitor.native_properties().scale_factor
        } else {
          1.
        };

        let workspace_width = self.to_rect()?.width();

        Ok(column_size_from_px(
          width.to_px(workspace_width, Some(scale_factor)),
          workspace_width,
          self.horizontal_inner_gap()?,
        ))
      }
    }
  }

  /// Converts a column width in pixels to a column size (fraction of the
  /// workspace width) for the scrolling layout.
  pub fn column_size_from_px(&self, width: i32) -> anyhow::Result<f32> {
    Ok(column_size_from_px(
      width,
      self.to_rect()?.width(),
      self.horizontal_inner_gap()?,
    ))
  }

  /// Default size of newly tiled columns in the scrolling layout.
  ///
  /// This is the size set via `set_columns_per_screen` if any, otherwise
  /// the configured default column width.
  pub fn default_column_size(&self) -> anyhow::Result<f32> {
    if let Some(column_size) = self.0.borrow().column_size_override {
      return Ok(column_size);
    }

    self.column_size(&self.layout_config().default_column_width)
  }

  /// Effective outer gaps for this workspace.
  ///
  /// Uses `single_window_outer_gap` when the workspace has a single tiling
  /// window, otherwise falls back to `outer_gap`.
  pub fn outer_gaps(&self) -> RectDelta {
    let is_single_window = self.tiling_children().nth(1).is_none();

    let gaps_config = &self.0.borrow().gaps_config;
    let gaps = if is_single_window {
      gaps_config
        .single_window_outer_gap
        .as_ref()
        .unwrap_or(&gaps_config.outer_gap)
    } else {
      &gaps_config.outer_gap
    };

    // TODO: Should this be scaled by the monitor's DPI?
    gaps.clone()
  }

  /// Gets the bounds of a workspace with the given outer gap config.
  fn workspace_rect_with_gap_config(
    &self,
    outer_gaps: &RectDelta,
  ) -> anyhow::Result<Rect> {
    let monitor =
      self.monitor().context("Workspace has no parent monitor.")?;

    let gaps_config = &self.0.borrow().gaps_config;
    let scale_factor = if gaps_config.scale_with_dpi {
      monitor.native_properties().scale_factor
    } else {
      1.
    };

    // Get the delta between the monitor's bounds and its working area.
    let monitor_bounds = monitor.native_properties().bounds;
    let working_area_delta = monitor
      .native_properties()
      .working_area
      .delta(&monitor_bounds);

    Ok(
      monitor_bounds
        // Scale the gaps if `scale_with_dpi` is enabled. Outer gap config
        // values can be a percentage (relative to the monitor bounds), so
        // the outer gap delta needs to be applied prior to the working
        // area delta.
        .apply_delta(&outer_gaps.inverse(), Some(scale_factor))
        .apply_delta(&working_area_delta, None),
    )
  }

  /// Gets the maximum bounds of a workspace considering both `outer_gap`
  /// and `single_window_outer_gap` config values.
  pub fn max_workspace_rect(&self) -> anyhow::Result<Rect> {
    let gaps_config = &self.0.borrow().gaps_config;

    // Get the workspace rect using `outer_gap`.
    let multi_window_rect =
      self.workspace_rect_with_gap_config(&gaps_config.outer_gap)?;

    let Some(single_gap) = &gaps_config.single_window_outer_gap else {
      return Ok(multi_window_rect);
    };

    // Get the workspace rect using `single_window_outer_gap`.
    let single_window_rect =
      self.workspace_rect_with_gap_config(single_gap)?;

    Ok(multi_window_rect.union(&single_window_rect))
  }

  pub fn to_dto(&self) -> anyhow::Result<ContainerDto> {
    let rect = self.to_rect()?;
    let config = self.config();

    let children = self
      .children()
      .iter()
      .map(CommonGetters::to_dto)
      .try_collect()?;

    Ok(ContainerDto::Workspace(WorkspaceDto {
      id: self.id(),
      name: config.name,
      display_name: config.display_name,
      parent_id: self.parent().map(|parent| parent.id()),
      children,
      child_focus_order: self.0.borrow().child_focus_order.clone().into(),
      has_focus: self.has_focus(None),
      is_displayed: self.is_displayed(),
      width: rect.width(),
      height: rect.height(),
      x: rect.x(),
      y: rect.y(),
      tiling_direction: self.tiling_direction(),
    }))
  }
}

impl_container_debug!(Workspace);
impl_common_getters!(Workspace);
impl_tiling_direction_getters!(Workspace);

impl PositionGetters for Workspace {
  fn to_rect(&self) -> anyhow::Result<Rect> {
    self.workspace_rect_with_gap_config(&self.outer_gaps())
  }
}

impl std::fmt::Display for Workspace {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Workspace(name={}, tiling_direction={:?})",
      self.config().name,
      self.tiling_direction(),
    )
  }
}

#[cfg(test)]
mod tests {
  use std::time::{Duration, Instant};

  use wm_common::{LayoutConfig, LayoutType};

  use crate::{
    commands::container::{
      attach_container, detach_container, set_focused_descendant,
    },
    models::{Monitor, TilingWindow, Workspace},
    test_utils::MOCK_MONITOR_WIDTH,
    traits::{CommonGetters, PositionGetters, TilingSizeGetters},
  };

  fn scrolling_layout() -> LayoutConfig {
    LayoutConfig {
      layout_type: LayoutType::Scrolling,
      ..LayoutConfig::default()
    }
  }

  #[test]
  fn scrolling_columns_keep_size_on_attach_and_detach() {
    let windows = [
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.3).call(),
      TilingWindow::mock().tiling_size(0.7).call(),
    ];

    let _workspace = Workspace::mock()
      .layout_config(scrolling_layout())
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    assert!((windows[0].tiling_size() - 0.5).abs() < f32::EPSILON);
    assert!((windows[1].tiling_size() - 0.3).abs() < f32::EPSILON);
    assert!((windows[2].tiling_size() - 0.7).abs() < f32::EPSILON);

    detach_container(windows[1].clone().into()).unwrap();

    assert!((windows[0].tiling_size() - 0.5).abs() < f32::EPSILON);
    assert!((windows[2].tiling_size() - 0.7).abs() < f32::EPSILON);
  }

  #[test]
  fn scrolling_viewport_follows_focused_column() {
    let windows = [
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
    ];

    let workspace = Workspace::mock()
      .layout_config(scrolling_layout())
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    let _monitor =
      Monitor::mock().workspaces(vec![workspace.clone()]).call();

    let half_width = MOCK_MONITOR_WIDTH / 2;

    // Initially, the first two columns fill the viewport.
    set_focused_descendant(&windows[0].clone().into(), None);
    assert!(!workspace.sync_scroll_offset().unwrap());
    assert!(!workspace.commit_scroll_offset(None));
    assert!(workspace.is_column_visible(windows[1].id(), false, false));
    assert!(!workspace.is_column_visible(windows[2].id(), true, true));

    // Focusing the third column scrolls it into view.
    set_focused_descendant(&windows[2].clone().into(), None);
    assert!(workspace.sync_scroll_offset().unwrap());
    assert_eq!(workspace.scroll_offset(), half_width);

    // Columns are displayed at the previous offset until committed.
    assert!(workspace.is_column_visible(windows[0].id(), false, false));
    assert!(workspace.commit_scroll_offset(None));

    assert!(!workspace.is_column_visible(windows[0].id(), true, true));
    assert!(workspace.is_column_visible(windows[2].id(), false, false));

    let rect = windows[2].to_rect().unwrap();
    assert_eq!(rect.x(), half_width);
    assert_eq!(rect.width(), half_width);
  }

  #[test]
  fn balance_evenly_sizes_columns_in_view() {
    let windows = [
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
    ];

    let workspace = Workspace::mock()
      .layout_config(scrolling_layout())
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    let _monitor =
      Monitor::mock().workspaces(vec![workspace.clone()]).call();

    // Centering column 1 cuts off columns 0 and 2 at the edges, and
    // leaves column 3 out of view.
    set_focused_descendant(&windows[1].clone().into(), None);
    workspace.center_column(windows[1].id()).unwrap();
    workspace.balance_columns_in_view().unwrap();
    workspace.commit_scroll_offset(None);

    for window in &windows[..3] {
      assert!((window.tiling_size() - 1.0 / 3.0).abs() < f32::EPSILON);
      assert!(workspace.is_column_visible(window.id(), false, false));
    }

    assert!((windows[3].tiling_size() - 0.5).abs() < f32::EPSILON);
    assert_eq!(workspace.scroll_offset(), 0);
  }

  #[test]
  fn set_columns_per_screen_resizes_all_columns() {
    let windows = [
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
    ];

    let workspace = Workspace::mock()
      .layout_config(scrolling_layout())
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    let _monitor =
      Monitor::mock().workspaces(vec![workspace.clone()]).call();

    // Scroll such that columns 2 and 3 are in view.
    set_focused_descendant(&windows[3].clone().into(), None);
    workspace.sync_scroll_offset().unwrap();

    workspace.set_columns_per_screen(3).unwrap();
    workspace.commit_scroll_offset(None);

    for window in &windows {
      assert!((window.tiling_size() - 1.0 / 3.0).abs() < f32::EPSILON);
    }

    // Column 2 can't be aligned with the left edge without leaving empty
    // space on the right, so columns 1-3 fill the viewport.
    assert!(!workspace.is_column_visible(windows[0].id(), true, true));

    for window in &windows[1..] {
      assert!(workspace.is_column_visible(window.id(), false, false));
    }

    // New columns are opened with the same size.
    assert!(
      (workspace.default_column_size().unwrap() - 1.0 / 3.0).abs()
        < f32::EPSILON
    );
  }

  #[test]
  fn centered_column_stays_centered_when_new_column_fits() {
    let windows = [
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
    ];

    let workspace = Workspace::mock()
      .layout_config(scrolling_layout())
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    let _monitor =
      Monitor::mock().workspaces(vec![workspace.clone()]).call();

    workspace.set_columns_per_screen(3).unwrap();
    set_focused_descendant(&windows[1].clone().into(), None);
    workspace.center_column(windows[1].id()).unwrap();
    workspace.sync_scroll_offset().unwrap();
    let centered_offset = workspace.scroll_offset();

    // Open a new column to the right of the centered column.
    let new_window = TilingWindow::mock()
      .tiling_size(workspace.default_column_size().unwrap())
      .call();

    attach_container(
      &new_window.clone().into(),
      &workspace.clone().into(),
      None,
    )
    .unwrap();

    set_focused_descendant(&new_window.clone().into(), None);
    workspace.sync_scroll_offset().unwrap();

    assert_eq!(workspace.scroll_offset(), centered_offset);
  }

  /// Creates a scrolling workspace with two half-width columns, where the
  /// second column is focused and centered.
  fn workspace_with_centered_column(
  ) -> (Workspace, Monitor, [TilingWindow; 2]) {
    let windows = [
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
    ];

    let workspace = Workspace::mock()
      .layout_config(scrolling_layout())
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    let monitor =
      Monitor::mock().workspaces(vec![workspace.clone()]).call();

    set_focused_descendant(&windows[1].clone().into(), None);
    workspace.center_column(windows[1].id()).unwrap();
    workspace.sync_scroll_offset().unwrap();

    (workspace, monitor, windows)
  }

  /// Opens a new half-width column to the right of the focused column,
  /// and focuses it.
  fn open_column(workspace: &Workspace) -> TilingWindow {
    let window = TilingWindow::mock().tiling_size(0.5).call();

    attach_container(
      &window.clone().into(),
      &workspace.clone().into(),
      None,
    )
    .unwrap();

    set_focused_descendant(&window.clone().into(), None);
    workspace.sync_scroll_offset().unwrap();

    window
  }

  #[test]
  fn closing_new_column_restores_previous_scroll_offset() {
    let (workspace, _monitor, windows) = workspace_with_centered_column();
    let centered_offset = workspace.scroll_offset();

    let new_window = open_column(&workspace);
    assert_ne!(workspace.scroll_offset(), centered_offset);

    detach_container(new_window.into()).unwrap();
    set_focused_descendant(&windows[1].clone().into(), None);
    workspace.sync_scroll_offset().unwrap();

    assert_eq!(workspace.scroll_offset(), centered_offset);
  }

  #[test]
  fn focus_change_discards_scroll_restore() {
    let (workspace, _monitor, windows) = workspace_with_centered_column();
    let centered_offset = workspace.scroll_offset();

    let new_window = open_column(&workspace);

    // Focus away from and back to the new column.
    set_focused_descendant(&windows[1].clone().into(), None);
    workspace.sync_scroll_offset().unwrap();
    set_focused_descendant(&new_window.clone().into(), None);
    workspace.sync_scroll_offset().unwrap();
    let offset_after_focus = workspace.scroll_offset();

    detach_container(new_window.into()).unwrap();
    set_focused_descendant(&windows[1].clone().into(), None);
    workspace.sync_scroll_offset().unwrap();

    assert_ne!(offset_after_focus, centered_offset);
    assert_ne!(workspace.scroll_offset(), centered_offset);
  }

  #[test]
  fn scroll_animation_moves_displayed_offset_towards_target() {
    let windows = [
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
      TilingWindow::mock().tiling_size(0.5).call(),
    ];

    let workspace = Workspace::mock()
      .layout_config(scrolling_layout())
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    let _monitor =
      Monitor::mock().workspaces(vec![workspace.clone()]).call();

    set_focused_descendant(&windows[2].clone().into(), None);
    workspace.sync_scroll_offset().unwrap();

    let start = Instant::now();
    let duration = Duration::from_millis(100);
    let target = workspace.scroll_offset();
    assert!(workspace.commit_scroll_offset(Some((start, duration))));
    assert!(!workspace.commit_scroll_offset(Some((start, duration))));

    // Columns are displayed at the start offset until the first frame.
    assert_eq!(workspace.displayed_scroll_offset(), 0);
    assert_eq!(windows[0].to_rect().unwrap().x(), 0);

    assert!(
      workspace.step_scroll_animation(start + Duration::from_millis(50))
    );
    let midway = workspace.displayed_scroll_offset();
    assert!(midway > 0 && midway < target);

    // Once finished, columns are displayed at the target offset.
    assert!(!workspace.step_scroll_animation(start + duration));
    assert!(!workspace.has_scroll_animation());
    assert_eq!(workspace.displayed_scroll_offset(), target);
  }
}
