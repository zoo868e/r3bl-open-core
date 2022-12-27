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
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{borrow::Cow, fmt::Debug};

use int_enum::IntEnum;
use r3bl_rs_utils_core::*;

use crate::*;

#[derive(Debug)]
pub enum DialogEngineApplyResponse {
    UpdateEditorBuffer(EditorBuffer),
    DialogChoice(DialogChoice),
    Noop,
}

/// Things you can do with a [DialogEngine].
impl DialogEngine {
    pub async fn render_engine<S, A>(
        args: DialogEngineArgs<'_, S, A>,
    ) -> CommonResult<RenderPipeline>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        let current_box: FlexBox = {
            match &args.dialog_engine.maybe_flex_box {
                // No need to calculate new flex box if the window size hasn't changed & there's an
                // existing one.
                Some((saved_window_size, saved_flex_box))
                    if saved_window_size == args.window_size =>
                {
                    saved_flex_box.clone()
                }
                // Otherwise, calculate a new flex box & save it.
                _ => {
                    let it = internal_impl::make_flex_box_for_dialog(
                        &args.self_id,
                        &args.dialog_engine.dialog_options.mode,
                        args.window_size,
                        None,
                    )?;
                    args.dialog_engine
                        .maybe_flex_box
                        .replace((*args.window_size, it.clone()));
                    it
                }
            }
        };

        let (origin_pos, bounds_size) =
            EditorEngineFlexBox::from(current_box).get_style_adjusted_position_and_size();

        let pipeline = {
            let mut it = render_pipeline!();

            it.push(
                ZOrder::Glass,
                internal_impl::render_border(&origin_pos, &bounds_size, args.dialog_engine),
            );

            it.push(
                ZOrder::Glass,
                internal_impl::render_title(
                    &origin_pos,
                    &bounds_size,
                    &args.dialog_buffer.title,
                    args.dialog_engine,
                ),
            );

            it += internal_impl::render_editor(&origin_pos, &bounds_size, args).await?;

            it
        };

        Ok(pipeline)
    }

    /// Event based interface for the editor. This executes the [InputEvent].
    /// 1. Returns [Some(DialogResponse)] if <kbd>Enter</kbd> or <kbd>Esc</kbd> was pressed.
    /// 2. Otherwise returns [None].
    pub async fn apply_event<S, A>(
        args: DialogEngineArgs<'_, S, A>,
        input_event: &InputEvent,
    ) -> CommonResult<DialogEngineApplyResponse>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        let DialogEngineArgs {
            self_id,
            component_registry,
            shared_store,
            shared_global_data,
            state,
            dialog_buffer,
            dialog_engine,
            ..
        } = args;

        if let Some(choice) = internal_impl::try_handle_dialog_choice(input_event, dialog_buffer) {
            return Ok(DialogEngineApplyResponse::DialogChoice(choice));
        }

        let editor_engine_args = EditorEngineArgs {
            component_registry,
            shared_global_data,
            self_id,
            editor_buffer: &dialog_buffer.editor_buffer,
            editor_engine: &mut dialog_engine.editor_engine,
            shared_store,
            state,
        };

        if let EditorEngineApplyResponse::Applied(new_editor_buffer) =
            EditorEngine::apply_event(editor_engine_args, input_event).await?
        {
            return Ok(DialogEngineApplyResponse::UpdateEditorBuffer(
                new_editor_buffer,
            ));
        }

        Ok(DialogEngineApplyResponse::Noop)
    }
}

mod internal_impl {
    use super::*;

    /// Return the [FlexBox] for the dialog to be rendered in.
    ///
    /// - In non-modal contexts (which this is not), this is determined by the layout engine.
    /// - In the modal case (which this is), things are different because the dialog escapes the
    ///   boundaries of the layout engine and really just paints itself on top of everything. It can
    ///   reach any corner of the screen.
    ///   - In autocomplete mode it sizes itself differently than in normal mode.
    /// - However, it is still constrained by the bounds of the [Surface] itself and does not take
    ///   into account the full window size (in case these are different). This only applies if a
    ///   [Surface] is passed in as an argument.
    ///
    /// ```text
    /// EditorEngineFlexBox {
    ///   id: ..,
    ///   style_adjusted_origin_pos: ..,
    ///   style_adjusted_bounds_size: ..,
    ///   maybe_computed_style: None,
    /// }
    /// ```
    pub fn make_flex_box_for_dialog(
        dialog_id: &FlexBoxId,
        _mode: &DialogEngineMode,
        window_size: &Size,
        maybe_surface: Option<&Surface>,
    ) -> CommonResult<FlexBox> {
        let surface_size = if let Some(surface) = maybe_surface {
            surface.box_size
        } else {
            *window_size
        };

        let surface_origin_pos = if let Some(surface) = maybe_surface {
            surface.origin_pos
        } else {
            position!(col_index: 0, row_index: 0)
        };

        // Check to ensure that the dialog box has enough space to be displayed.
        if window_size.col_count < ch!(MinSize::Col.int_value())
            || window_size.row_count < ch!(MinSize::Row.int_value())
        {
            return CommonError::new(
                CommonErrorType::DisplaySizeTooSmall,
                &format!(
                    "Window size is too small. Min size is {} cols x {} rows",
                    MinSize::Col.int_value(),
                    MinSize::Row.int_value()
                ),
            );
        }

        // TODO: use `_mode` in order to create the correct dialog_size

        let dialog_size = {
            // Calc dialog bounds size based on window size.
            let size = size! { col_count: surface_size.col_count * 90/100, row_count: 4 };
            assert!(size.row_count < ch!(MinSize::Row.int_value()));
            size
        };

        let mut origin_pos = {
            // Calc origin position based on window size & dialog size.
            let origin_col = surface_size.col_count / 2 - dialog_size.col_count / 2;
            let origin_row = surface_size.row_count / 2 - dialog_size.row_count / 2;
            position!(col_index: origin_col, row_index: origin_row)
        };
        origin_pos += surface_origin_pos;

        throws_with_return!({
            EditorEngineFlexBox {
                id: *dialog_id,
                style_adjusted_origin_pos: origin_pos,
                style_adjusted_bounds_size: dialog_size,
                maybe_computed_style: None,
            }
            .into()
        })
    }

    pub async fn render_editor<S, A>(
        origin_pos: &Position,
        bounds_size: &Size,
        args: DialogEngineArgs<'_, S, A>,
    ) -> CommonResult<RenderPipeline>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        let maybe_style = args.dialog_engine.dialog_options.maybe_style_editor.clone();

        let flex_box: FlexBox = EditorEngineFlexBox {
            id: args.self_id,
            style_adjusted_origin_pos: position! {col_index: origin_pos.col_index + 1, row_index: origin_pos.row_index + 2},
            style_adjusted_bounds_size: size! {col_count: bounds_size.col_count - 2, row_count: 1},
            maybe_computed_style: maybe_style,
        }
        .into();

        let editor_engine_args = EditorEngineArgs {
            component_registry: args.component_registry,
            shared_global_data: args.shared_global_data,
            self_id: args.self_id,
            editor_buffer: &args.dialog_buffer.editor_buffer,
            editor_engine: &mut args.dialog_engine.editor_engine,
            shared_store: args.shared_store,
            state: args.state,
        };

        let mut pipeline = EditorEngine::render_engine(editor_engine_args, &flex_box).await?;
        pipeline.hoist(ZOrder::Normal, ZOrder::Glass);

        Ok(pipeline)
    }

    pub fn render_title(
        origin_pos: &Position,
        bounds_size: &Size,
        title: &str,
        dialog_engine: &mut DialogEngine,
    ) -> RenderOps {
        let mut ops = render_ops!();

        let row_pos =
            position!(col_index: origin_pos.col_index + 1, row_index: origin_pos.row_index + 1);
        let unicode_string = UnicodeString::from(title);
        let mut text_content = Cow::Borrowed(unicode_string.truncate_to_fit_size(size! {
          col_count: bounds_size.col_count - 2, row_count: bounds_size.row_count
        }));

        // Apply lolcat override (if enabled) to the fg_color of text_content.
        apply_lolcat_from_style(
            &dialog_engine.dialog_options.maybe_style_title,
            &mut dialog_engine.lolcat,
            &mut text_content,
        );

        ops.push(RenderOp::ResetColor);
        ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
        ops.push(RenderOp::ApplyColors(
            dialog_engine.dialog_options.maybe_style_title.clone(),
        ));
        ops.push(RenderOp::PaintTextWithAttributes(
            text_content.into(),
            dialog_engine.dialog_options.maybe_style_title.clone(),
        ));

        ops
    }

    pub fn render_border(
        origin_pos: &Position,
        bounds_size: &Size,
        dialog_engine: &mut DialogEngine,
    ) -> RenderOps {
        let mut ops = render_ops!();

        let inner_spaces = SPACER.repeat(ch!(@to_usize bounds_size.col_count - 2));

        let maybe_style = dialog_engine.dialog_options.maybe_style_border.clone();

        for row_idx in 0..*bounds_size.row_count {
            let row_pos = position!(col_index: origin_pos.col_index, row_index: origin_pos.row_index + row_idx);

            let is_first_line = row_idx == 0;
            let is_last_line = row_idx == (*bounds_size.row_count - 1);

            ops.push(RenderOp::ResetColor);
            ops.push(RenderOp::MoveCursorPositionAbs(row_pos));
            ops.push(RenderOp::ApplyColors(maybe_style.clone()));

            match (is_first_line, is_last_line) {
                // First line.
                (true, false) => {
                    let mut text_content = Cow::Owned(format!(
                        "{}{}{}",
                        BorderGlyphCharacter::TopLeft.as_ref(),
                        BorderGlyphCharacter::Horizontal
                            .as_ref()
                            .repeat(ch!(@to_usize bounds_size.col_count - 2)),
                        BorderGlyphCharacter::TopRight.as_ref()
                    ));
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    apply_lolcat_from_style(
                        &maybe_style,
                        &mut dialog_engine.lolcat,
                        &mut text_content,
                    );

                    ops.push(RenderOp::PaintTextWithAttributes(
                        text_content.into(),
                        maybe_style.clone(),
                    ));
                }
                // Last line.
                (false, true) => {
                    let mut text_content = Cow::Owned(format!(
                        "{}{}{}",
                        BorderGlyphCharacter::BottomLeft.as_ref(),
                        BorderGlyphCharacter::Horizontal
                            .as_ref()
                            .repeat(ch!(@to_usize bounds_size.col_count - 2)),
                        BorderGlyphCharacter::BottomRight.as_ref(),
                    ));
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    apply_lolcat_from_style(
                        &maybe_style,
                        &mut dialog_engine.lolcat,
                        &mut text_content,
                    );
                    ops.push(RenderOp::PaintTextWithAttributes(
                        text_content.into(),
                        maybe_style.clone(),
                    ));
                }
                // Middle line.
                (false, false) => {
                    let mut text_content = Cow::Owned(format!(
                        "{}{}{}",
                        BorderGlyphCharacter::Vertical.as_ref(),
                        inner_spaces,
                        BorderGlyphCharacter::Vertical.as_ref()
                    ));
                    // Apply lolcat override (if enabled) to the fg_color of text_content.
                    apply_lolcat_from_style(
                        &maybe_style,
                        &mut dialog_engine.lolcat,
                        &mut text_content,
                    );
                    ops.push(RenderOp::PaintTextWithAttributes(
                        text_content.into(),
                        maybe_style.clone(),
                    ));
                }
                _ => {}
            };
        }

        ops
    }

    pub fn try_handle_dialog_choice(
        input_event: &InputEvent,
        dialog_buffer: &DialogBuffer,
    ) -> Option<DialogChoice> {
        match DialogEvent::from(input_event) {
            // Handle Enter.
            DialogEvent::EnterPressed => {
                let text = dialog_buffer.editor_buffer.get_as_string();
                return Some(DialogChoice::Yes(text));
            }

            // Handle Esc.
            DialogEvent::EscPressed => {
                return Some(DialogChoice::No);
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod test_dialog_engine_api_render_engine {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::test_dialog::mock_real_objects_for_dialog;

    #[test]
    fn test_make_flex_box_for_dialog() {
        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: position! { col_index: 2, row_index: 2 },
            box_size: size!( col_count: 65, row_count: 10 ),
            ..Default::default()
        };
        let window_size = size!( col_count: 70, row_count: 15 );
        let self_id: FlexBoxId = 0;

        // The dialog box should be centered inside the surface.
        let _result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            &self_id,
            &DialogEngineMode::ModalSimple,
            &window_size,
            Some(&surface),
        ))
        .unwrap();

        // TODO: impl this test
    }

    #[tokio::test]
    async fn render_engine() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let pipeline = dbg!(DialogEngine::render_engine(args).await.unwrap());
        assert_eq2!(pipeline.len(), 1);
        let render_ops = pipeline.get(&ZOrder::Glass).unwrap();
        assert!(!render_ops.is_empty());
    }
}

#[cfg(test)]
mod test_dialog_api_make_flex_box_for_dialog {
    use std::error::Error;

    use r3bl_rs_utils_core::*;

    use crate::{dialog_engine_api::internal_impl, *};

    /// More info on `is` and downcasting:
    /// - https://stackoverflow.com/questions/71409337/rust-how-to-match-against-any
    /// - https://ysantos.com/blog/downcast-rust
    #[test]
    fn make_flex_box_for_dialog_display_size_too_small() {
        let surface = Surface::default();
        let window_size = Size::default();
        let dialog_id: FlexBoxId = 0;

        // The window size is too small and will result in this error.
        // Err(
        //   CommonError {
        //       err_type: DisplaySizeTooSmall,
        //       err_msg: Some(
        //           "Window size is too small. Min size is 65 cols x 10 rows",
        //       ),
        //   },
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            &dialog_id,
            &DialogEngineMode::ModalSimple,
            &window_size,
            Some(&surface),
        ));

        // Assert that a general `CommonError` is returned.
        let my_err: Box<dyn Error + Send + Sync> = result_flex_box.err().unwrap();
        assert_eq2!(my_err.is::<CommonError>(), true);

        // Assert that this specific error is returned.
        let result = matches!(
            my_err.downcast_ref::<CommonError>(),
            Some(CommonError {
                err_type: CommonErrorType::DisplaySizeTooSmall,
                err_msg: _,
            })
        );

        assert_eq2!(result, true);
    }

    #[test]
    fn make_flex_box_for_dialog() {
        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: position! { col_index: 2, row_index: 2 },
            box_size: size!( col_count: 65, row_count: 10 ),
            ..Default::default()
        };
        let window_size = size!( col_count: 70, row_count: 15 );
        let self_id: FlexBoxId = 0;

        // The dialog box should be centered inside the surface.
        let result_flex_box = dbg!(internal_impl::make_flex_box_for_dialog(
            &self_id,
            &DialogEngineMode::ModalSimple,
            &window_size,
            Some(&surface),
        ));

        assert_eq2!(result_flex_box.is_ok(), true);

        let flex_box = result_flex_box.unwrap();
        assert_eq2!(flex_box.id, self_id);
        assert_eq2!(
            flex_box.style_adjusted_bounds_size,
            size!( col_count: 58, row_count: 4 )
        );
        assert_eq2!(
            flex_box.style_adjusted_origin_pos,
            position!( col_index: 5, row_index: 5 )
        );
    }
}

#[cfg(test)]
mod test_dialog_engine_api_apply_event {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::test_dialog::mock_real_objects_for_dialog;

    #[tokio::test]
    async fn apply_event_esc() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Esc));
        let response = dbg!(DialogEngine::apply_event(args, &input_event).await.unwrap());
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::No)
        ));
    }

    #[tokio::test]
    async fn apply_event_enter() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Enter));
        let response = dbg!(DialogEngine::apply_event(args, &input_event).await.unwrap());
        if let DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(value)) = &response {
            assert_eq2!(value, "");
        }
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(_))
        ));
    }

    #[tokio::test]
    async fn apply_event_other_key() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects_for_dialog::make_dialog_engine();
        let shared_store = &mock_real_objects_for_dialog::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects_for_editor::make_shared_global_data(
                (*window_size).into(),
            );
        let component_registry =
            &mut test_editor::mock_real_objects_for_editor::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@char 'a'));
        let response = dbg!(DialogEngine::apply_event(args, &input_event).await.unwrap());
        if let DialogEngineApplyResponse::UpdateEditorBuffer(editor_buffer) = &response {
            assert_eq2!(editor_buffer.get_as_string(), "a");
        }
    }
}
