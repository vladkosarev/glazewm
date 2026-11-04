use crate::{
  models::Workspace,
  traits::{CommonGetters, TilingSizeGetters},
  wm_state::WmState,
};

/// Resets the tiling size of all tiling containers in the workspace, such
/// that siblings within each direction container are evenly sized.
///
/// Applies recursively to all nested split containers. In the scrolling
/// layout, the columns within view are instead evenly sized to fill the
/// viewport.
pub fn balance_tiling_sizes(
  workspace: &Workspace,
  state: &mut WmState,
) -> anyhow::Result<()> {
  if workspace.is_scrolling() {
    workspace.balance_columns_in_view()?;
  } else {
    reset_tiling_sizes(workspace);
  }

  state
    .pending_sync
    .queue_container_to_redraw(workspace.clone());

  Ok(())
}

/// Sets the tiling size of each tiling container in the workspace to
/// `1 / n`, where `n` is the number of tiling siblings it has (including
/// itself).
fn reset_tiling_sizes(workspace: &Workspace) {
  for container in workspace.self_and_descendants() {
    let tiling_children = container.tiling_children().collect::<Vec<_>>();

    #[allow(clippy::cast_precision_loss)]
    let tiling_size = 1.0 / tiling_children.len() as f32;

    for child in &tiling_children {
      child.set_tiling_size(tiling_size);
    }
  }
}

#[cfg(test)]
mod tests {
  use wm_common::TilingDirection;

  use super::reset_tiling_sizes;
  use crate::{
    models::{SplitContainer, TilingWindow, Workspace},
    traits::TilingSizeGetters,
  };

  #[test]
  fn balances_flat_workspace() {
    let windows = [
      TilingWindow::mock().call(),
      TilingWindow::mock().call(),
      TilingWindow::mock().call(),
    ];

    let workspace = Workspace::mock()
      .tiling_containers(windows.iter().cloned().map(Into::into).collect())
      .call();

    windows[0].set_tiling_size(0.1);
    windows[1].set_tiling_size(0.7);
    windows[2].set_tiling_size(0.2);

    reset_tiling_sizes(&workspace);

    for window in &windows {
      assert!((window.tiling_size() - 1.0 / 3.0).abs() < f32::EPSILON);
    }
  }

  #[test]
  fn balances_nested_split_containers() {
    let nested_windows =
      [TilingWindow::mock().call(), TilingWindow::mock().call()];

    let split = SplitContainer::mock()
      .tiling_direction(TilingDirection::Vertical)
      .tiling_containers(
        nested_windows.iter().cloned().map(Into::into).collect(),
      )
      .call();

    let window = TilingWindow::mock().call();

    let workspace = Workspace::mock()
      .tiling_containers(vec![window.clone().into(), split.clone().into()])
      .call();

    window.set_tiling_size(0.8);
    split.set_tiling_size(0.2);
    nested_windows[0].set_tiling_size(0.9);
    nested_windows[1].set_tiling_size(0.1);

    reset_tiling_sizes(&workspace);

    assert!((window.tiling_size() - 0.5).abs() < f32::EPSILON);
    assert!((split.tiling_size() - 0.5).abs() < f32::EPSILON);

    for nested_window in &nested_windows {
      assert!((nested_window.tiling_size() - 0.5).abs() < f32::EPSILON);
    }
  }
}
