use crate::{
  models::{Container, Workspace},
  traits::CommonGetters,
  wm_state::WmState,
};

/// Sizes all columns of the container's workspace such that the given
/// number of columns fill the screen.
///
/// Has no effect outside of the scrolling layout.
pub fn set_columns_per_screen(
  container: &Container,
  count: u32,
  state: &mut WmState,
) -> anyhow::Result<()> {
  let Some(workspace) =
    container.workspace().filter(Workspace::is_scrolling)
  else {
    return Ok(());
  };

  workspace.set_columns_per_screen(count)?;

  state
    .pending_sync
    .queue_containers_to_redraw(workspace.tiling_children());

  Ok(())
}
