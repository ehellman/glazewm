use std::{
  cell::{Ref, RefCell, RefMut},
  collections::VecDeque,
  rc::Rc,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  common::{Rect, RectDelta, TilingDirection},
  containers::{
    traits::{CommonGetters, PositionGetters, TilingDirectionGetters},
    Container, ContainerDto, DirectionContainer, TilingContainer,
    WindowContainer,
  },
  impl_common_getters, impl_container_debug,
  impl_tiling_direction_getters,
  user_config::WorkspaceConfig,
};

#[derive(Clone)]
pub struct Workspace(Rc<RefCell<WorkspaceInner>>);

#[derive(Debug)]
struct WorkspaceInner {
  id: Uuid,
  name: String,
  parent: Option<Container>,
  children: VecDeque<Container>,
  child_focus_order: VecDeque<Uuid>,
  tiling_direction: TilingDirection,
  config: WorkspaceConfig,
  outer_gap: RectDelta,
}

/// User-friendly representation of a workspace.
///
/// Used for IPC and debug logging.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
  id: Uuid,
  name: String,
  parent: Option<Uuid>,
  children: Vec<ContainerDto>,
  child_focus_order: Vec<Uuid>,
  is_displayed: bool,
  width: i32,
  height: i32,
  x: i32,
  y: i32,
  tiling_direction: TilingDirection,
}

impl Workspace {
  pub fn new(
    config: WorkspaceConfig,
    outer_gap: RectDelta,
    tiling_direction: TilingDirection,
  ) -> Self {
    let workspace = WorkspaceInner {
      id: Uuid::new_v4(),
      name: config.name.clone(),
      parent: None,
      children: VecDeque::new(),
      child_focus_order: VecDeque::new(),
      tiling_direction,
      config,
      outer_gap,
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
      .map(|workspace| workspace.id() == self.id())
      .unwrap_or(false)
  }

  pub fn set_outer_gap(&self, outer_gap: RectDelta) {
    self.0.borrow_mut().outer_gap = outer_gap;
  }

  pub fn to_dto(&self) -> anyhow::Result<ContainerDto> {
    let rect = self.to_rect()?;
    let children = self
      .children()
      .iter()
      .map(|child| child.to_dto())
      .try_collect()?;

    Ok(ContainerDto::Workspace(WorkspaceDto {
      id: self.id(),
      name: self.0.borrow().name.clone(),
      parent: self.parent().map(|parent| parent.id()),
      children,
      child_focus_order: self.0.borrow().child_focus_order.clone().into(),
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
    let working_rect = self
      .monitor()
      .context("Workspace has no parent monitor.")?
      .native()
      .working_rect()
      .cloned()
      .context("Failed to get working area of parent monitor.")?;

    let outer_gap = &self.0.borrow().outer_gap;
    Ok(working_rect.apply_inverse_delta(outer_gap))
  }
}
