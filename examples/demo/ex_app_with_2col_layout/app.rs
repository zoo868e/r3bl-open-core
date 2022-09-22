/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed &to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use r3bl_rs_utils::*;
use tokio::sync::RwLock;

use super::*;

// Constants for the ids.
const CONTAINER_ID: &str = "container";
const COL_1_ID: &str = "col_1";
const COL_2_ID: &str = "col_2";

/// Async trait object that implements the [TWApp] trait.
#[derive(Default)]
pub struct AppWithLayout {
  pub component_registry: ComponentRegistry<State, Action>,
  pub has_focus: HasFocus,
}

mod app_impl {
  use super::*;

  #[async_trait]
  impl App<State, Action> for AppWithLayout {
    async fn app_handle_event(
      &mut self, input_event: &InputEvent, state: &State,
      shared_store: &SharedStore<State, Action>, _terminal_size: Size,
    ) -> CommonResult<EventPropagation> {
      // Try to handle left and right arrow key input events & return if handled.
      if let Continuation::Return = self.handle_focus_switch(input_event) {
        return Ok(EventPropagation::ConsumedRerender);
      }

      // Route any unhandled event to the component that has focus.
      route_event_to_focused_component!(
        registry:     self.component_registry,
        has_focus:    self.has_focus,
        input_event:  input_event,
        state:        state,
        shared_store: shared_store
      )
    }

    async fn app_render(
      &mut self, state: &State, shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<RenderPipeline> {
      throws_with_return!({
        let window_size = shared_tw_data.read().await.get_size();
        // Render container component.
        let mut surface = surface_start_with_runnable! {
          runnable:       self,
          stylesheet:     style_helpers::create_stylesheet()?,
          pos:            position!(col:0, row:0),
          size:           size!(col: window_size.col, row: window_size.row - 1), // Bottom row for status bar.
          state:          state,
          shared_store:   shared_store,
          shared_tw_data: shared_tw_data
        };

        // Render status bar.
        status_bar_helpers::render(&mut surface.render_pipeline, window_size);

        // Return RenderOps pipeline (which will actually be painted elsewhere).
        surface.render_pipeline
      });
    }
  }

  #[async_trait]
  impl SurfaceRunnable<State, Action> for AppWithLayout {
    async fn run_on_surface(
      &mut self, surface: &mut Surface, state: &State, shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<()> {
      self.create_components_populate_registry_init_focus().await;
      self
        .create_main_container(surface, state, shared_store, shared_tw_data)
        .await
    }
  }
}

// Handle focus.
mod handle_focus {
  use super::*;

  impl AppWithLayout {
    pub fn handle_focus_switch(&mut self, input_event: &InputEvent) -> Continuation<String> {
      let mut event_consumed = false;

      // Handle Left, Right to switch focus between columns.
      if let InputEvent::Keyboard(keypress) = input_event {
        match keypress {
          Keypress::Plain {
            key: Key::SpecialKey(SpecialKey::Left),
          } => {
            event_consumed = true;
            self.switch_focus(SpecialKey::Left);
            debug_log_has_focus(
              stringify!(AppWithLayout::app_handle_event).into(),
              &self.has_focus,
            );
          }
          Keypress::Plain {
            key: Key::SpecialKey(SpecialKey::Right),
          } => {
            event_consumed = true;
            self.switch_focus(SpecialKey::Right);
            debug_log_has_focus(
              stringify!(AppWithLayout::app_handle_event).into(),
              &self.has_focus,
            );
          }
          _ => {}
        }
      }

      if event_consumed {
        Continuation::Return
      } else {
        Continuation::Continue
      }
    }

    fn switch_focus(&mut self, special_key: SpecialKey) {
      if let Some(_id) = self.has_focus.get_id() {
        if special_key == SpecialKey::Left {
          self.has_focus.set_id(COL_1_ID)
        } else {
          self.has_focus.set_id(COL_2_ID)
        }
      } else {
        log_no_err!(ERROR, "No focus id has been set, and it should be set!");
      }
    }
  }
}

// Handle component registry.
mod construct_components {
  use super::*;

  impl AppWithLayout {
    pub async fn create_components_populate_registry_init_focus(&mut self) {
      let _component = ColumnRenderComponent::default();
      let shared_component_r1 = Arc::new(RwLock::new(_component));
      let shared_component_r2 = shared_component_r1.clone();

      // Construct COL_1_ID.
      if self.component_registry.id_does_not_exist(COL_1_ID) {
        self.component_registry.put(COL_1_ID, shared_component_r1);
      }

      // Construct COL_2_ID.
      if self.component_registry.id_does_not_exist(COL_2_ID) {
        self.component_registry.put(COL_2_ID, shared_component_r2);
      }

      // Init has focus.
      if self.has_focus.get_id().is_none() {
        self.has_focus.set_id(COL_1_ID);
      }
    }

    /// Main container CONTAINER_ID.
    pub async fn create_main_container(
      &mut self, surface: &mut Surface, state: &State, shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<()> {
      throws!({
        box_start_with_runnable! {
          in:                     surface,
          runnable:               layout_components::TwoColLayout { app_with_layout: self },
          id:                     CONTAINER_ID,
          dir:                    Direction::Horizontal,
          requested_size_percent: requested_size_percent!(width: 100, height: 100),
          styles:                 [CONTAINER_ID],
          state:                  state,
          shared_store:           shared_store,
          shared_tw_data:         shared_tw_data
        };
      });
    }
  }
}

mod layout_components {
  use super::*;

  pub(crate) struct TwoColLayout<'a> {
    pub(crate) app_with_layout: &'a AppWithLayout,
  }

  #[async_trait]
  impl<'a> SurfaceRunnable<State, Action> for TwoColLayout<'a> {
    async fn run_on_surface(
      &mut self, surface: &mut Surface, state: &State, shared_store: &SharedStore<State, Action>,
      _shared_tw_data: &SharedTWData,
    ) -> CommonResult<()> {
      self.create_left_col(surface, state, shared_store).await?;
      self.create_right_col(surface, state, shared_store).await?;
      Ok(())
    }
  }

  impl<'a> TwoColLayout<'a> {
    /// Left column COL_1_ID.
    async fn create_left_col(
      &mut self, surface: &mut Surface, state: &State, shared_store: &SharedStore<State, Action>,
    ) -> CommonResult<()> {
      throws!({
        box_start_with_component! {
          in:                     surface,
          id:                     COL_1_ID,
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 50, height: 100),
          styles:                 [COL_1_ID],
          render: {
            from:         self.app_with_layout.component_registry,
            has_focus:    self.app_with_layout.has_focus,
            state:        state,
            shared_store: shared_store
          }
        }
      });
    }

    /// Right column COL_2_ID.
    async fn create_right_col(
      &mut self, surface: &mut Surface, state: &State, shared_store: &SharedStore<State, Action>,
    ) -> CommonResult<()> {
      throws!({
        box_start_with_component! {
          in:                     surface,
          id:                     COL_2_ID,
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 50, height: 100),
          styles:                 [COL_2_ID],
          render: {
            from:         self.app_with_layout.component_registry,
            has_focus:    self.app_with_layout.has_focus,
            state:        state,
            shared_store: shared_store
          }
        }
      });
    }
  }
}

mod debug_helpers {
  use super::*;

  impl Debug for AppWithLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("AppWithLayout")
        .field("component_registry", &self.component_registry)
        .field("state_manage_focus_data", &self.has_focus)
        .finish()
    }
  }
}

mod style_helpers {
  use super::*;

  pub fn create_stylesheet() -> CommonResult<Stylesheet> {
    throws_with_return!({
      stylesheet! {
        style! {
          id: CONTAINER_ID
          padding: 1
        },
        style! {
          id: COL_1_ID
          padding: 1
          color_bg: TWColor::Rgb { r: 55, g: 55, b: 100 }
        },
        style! {
          id: COL_2_ID
          padding: 1
          color_bg: TWColor::Rgb { r: 55, g: 55, b: 248 }
        }
      }
    })
  }
}

mod status_bar_helpers {
  use super::*;

  /// Shows helpful messages at the bottom row of the screen.
  pub fn render(render_pipeline: &mut RenderPipeline, size: Size) {
    let st_vec = styled_texts! {
      styled_text! { "Hints:",          style!(attrib: [dim])       },
      styled_text! { " x : Exit ⛔ ",   style!(attrib: [bold])      },
      styled_text! { " … ",             style!(attrib: [dim])       },
      styled_text! { " ↑ / + : inc ",   style!(attrib: [underline]) },
      styled_text! { " … ",             style!(attrib: [dim])       },
      styled_text! { " ↓ / - : dec ",   style!(attrib: [underline]) },
      styled_text! { " … ",             style!(attrib: [dim])       },
      styled_text! { " ← / → : focus ", style!(attrib: [underline]) }
    };

    let display_width = st_vec.display_width();
    let col_center: ChUnit = (size.col - display_width) / 2;
    let row_bottom: ChUnit = size.row - 1;
    let center: Position = position!(col: col_center, row: row_bottom);

    *render_pipeline += (ZOrder::Normal, RenderOp::MoveCursorPositionAbs(center));
    *render_pipeline += st_vec.render(ZOrder::Normal);
  }
}