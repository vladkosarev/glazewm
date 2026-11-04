use super::{move_container_within_tree, ColumnEnd};
use crate::{
  models::{Container, Workspace},
  traits::CommonGetters,
  wm_state::WmState,
};

/// Moves a column of the scrolling layout to the first or last position
/// within its workspace.
///
/// Has no effect outside of the scrolling layout.
pub fn move_column_to_end(
  container: &Container,
  column_end: ColumnEnd,
  state: &mut WmState,
) -> anyhow::Result<()> {
  let Some(workspace) = container
    .parent()
    .and_then(|parent| parent.as_workspace().cloned())
    .filter(Workspace::is_scrolling)
  else {
    return Ok(());
  };

  if !container.is_tiling_window() {
    return Ok(());
  }

  let target_index = match column_end {
    ColumnEnd::First => 0,
    ColumnEnd::Last => workspace.child_count(),
  };

  move_container_within_tree(
    container,
    &workspace.clone().into(),
    target_index,
    state,
  )?;

  state
    .pending_sync
    .queue_containers_to_redraw(workspace.tiling_children());

  Ok(())
}
