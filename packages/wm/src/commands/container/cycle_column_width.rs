use super::resize_tiling_container;
use crate::{
  models::Container,
  traits::{CommonGetters, TilingSizeGetters},
  wm_state::WmState,
};

/// Tolerance when comparing a column's size against the preset sizes.
const SIZE_EPSILON: f32 = 0.001;

/// Cycles the width of a column in the scrolling layout through the preset
/// column widths.
///
/// The column is resized to the smallest preset that is larger than its
/// current width, wrapping around to the smallest preset.
///
/// Has no effect outside of the scrolling layout.
pub fn cycle_column_width(
  container: &Container,
  state: &mut WmState,
) -> anyhow::Result<()> {
  let Ok(column) = container.as_tiling_container() else {
    return Ok(());
  };

  let Some(workspace) = column.scrolling_workspace() else {
    return Ok(());
  };

  let mut preset_sizes = workspace
    .layout_config()
    .preset_column_widths
    .iter()
    .map(|width| workspace.column_size(width))
    .collect::<anyhow::Result<Vec<_>>>()?;

  preset_sizes.sort_by(f32::total_cmp);

  let current_size = column.tiling_size();
  let target_size = preset_sizes
    .iter()
    .find(|&&size| size > current_size + SIZE_EPSILON)
    .or_else(|| preset_sizes.first());

  if let Some(&target_size) = target_size {
    resize_tiling_container(&column, target_size);

    state
      .pending_sync
      .queue_containers_to_redraw(workspace.tiling_children());
  }

  Ok(())
}
