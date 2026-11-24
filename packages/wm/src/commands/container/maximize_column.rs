use super::resize_tiling_container;
use crate::{
  models::Container,
  traits::{CommonGetters, TilingSizeGetters},
  wm_state::WmState,
};

/// Toggles whether a column of the scrolling layout fills the viewport.
///
/// Unlike fullscreen, the column stays part of the layout, with gaps
/// around it. Toggling again restores the column's previous width.
///
/// Has no effect outside of the scrolling layout.
pub fn maximize_column(container: &Container, state: &mut WmState) {
  let Ok(column) = container.as_tiling_container() else {
    return;
  };

  let Some(workspace) = column.scrolling_workspace() else {
    return;
  };

  let target_size =
    workspace.toggle_column_maximized(column.id(), column.tiling_size());

  if let Some(target_size) = target_size {
    resize_tiling_container(&column, target_size);

    state
      .pending_sync
      .queue_containers_to_redraw(workspace.tiling_children());
  }
}
