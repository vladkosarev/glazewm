use std::{
  cell::{Ref, RefCell, RefMut},
  collections::VecDeque,
  rc::Rc,
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
    column_size_from_px, Container, DirectionContainer, ScrollingStrip,
    TilingContainer, WindowContainer,
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
  /// scrolling layout.
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
  pub fn scroll_offset(&self) -> i32 {
    self.0.borrow().scroll_offset
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
  /// account the scroll offset.
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
      workspace_rect.x() + start - self.scroll_offset(),
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
          self.scroll_offset(),
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

    // Get indices of the most and second-most recently focused columns.
    let mut focus_order = self
      .tiling_children()
      .enumerate()
      .map(|(index, child)| (child.focus_index(), index))
      .collect::<Vec<_>>();

    focus_order.sort_unstable();
    let mut focused_columns = focus_order.into_iter().map(|(_, i)| i);

    let scroll_offset = strip.scroll_offset_for_focus(
      focused_columns.next(),
      focused_columns.next(),
      self.scroll_offset(),
      &self.0.borrow().layout_config.center_focused_column,
    );

    let has_changed = scroll_offset != self.scroll_offset();
    self.0.borrow_mut().scroll_offset = scroll_offset;

    Ok(has_changed)
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
  pub fn default_column_size(&self) -> anyhow::Result<f32> {
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
  use wm_common::{LayoutConfig, LayoutType};

  use crate::{
    commands::container::{detach_container, set_focused_descendant},
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
    assert!(workspace.is_column_visible(windows[1].id(), false, false));
    assert!(!workspace.is_column_visible(windows[2].id(), true, true));

    // Focusing the third column scrolls it into view.
    set_focused_descendant(&windows[2].clone().into(), None);
    assert!(workspace.sync_scroll_offset().unwrap());
    assert_eq!(workspace.scroll_offset(), half_width);
    assert!(!workspace.is_column_visible(windows[0].id(), true, true));
    assert!(workspace.is_column_visible(windows[2].id(), false, false));

    let rect = windows[2].to_rect().unwrap();
    assert_eq!(rect.x(), half_width);
    assert_eq!(rect.width(), half_width);
  }
}
