use crate::{
  models::Container,
  traits::{CommonGetters, TilingSizeGetters},
  wm_state::WmState,
};

/// Scrolls the viewport of the scrolling layout to center the given
/// column.
///
/// Has no effect outside of the scrolling layout.
pub fn center_column(
  container: &Container,
  state: &mut WmState,
) -> anyhow::Result<()> {
  let Ok(column) = container.as_tiling_container() else {
    return Ok(());
  };

  let Some(workspace) = column.scrolling_workspace() else {
    return Ok(());
  };

  workspace.center_column(column.id())?;

  state
    .pending_sync
    .queue_containers_to_redraw(workspace.tiling_children());

  Ok(())
}
