use super::set_focused_descendant;
use crate::{
  models::{Container, Workspace},
  traits::CommonGetters,
  wm_state::WmState,
};

/// Either end of the row of columns in the scrolling layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnEnd {
  First,
  Last,
}

/// Focuses the first or last column of the container's workspace.
///
/// Has no effect outside of the scrolling layout.
pub fn focus_column_end(
  container: &Container,
  column_end: ColumnEnd,
  state: &mut WmState,
) {
  let Some(workspace) =
    container.workspace().filter(Workspace::is_scrolling)
  else {
    return;
  };

  let mut columns = workspace.tiling_children();

  let target = match column_end {
    ColumnEnd::First => columns.next(),
    ColumnEnd::Last => columns.last(),
  };

  if let Some(target) = target {
    set_focused_descendant(&target.into(), None);
    state.pending_sync.queue_focus_change().queue_cursor_jump();
  }
}
