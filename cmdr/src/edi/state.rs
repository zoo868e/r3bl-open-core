/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::{collections::HashMap, fmt::*};

use r3bl_tui::*;

use crate::edi::Id;

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

#[cfg(test)]
mod state_tests {
    use r3bl_tui::{generate_random_friendly_id, FlexBoxId};

    use crate::edi::Id;

    #[test]
    fn test_file_extension() {
        let file_path = Some("foo.rs".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "rs");

        let file_path = Some("foo".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.bar.rs".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "rs");

        let file_path = Some("foo.bar".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "bar");

        let file_path = Some("foo.bar.".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.bar.baz".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "baz");

        let file_path = Some("foo.bar.baz.".to_string());
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = None;
        let file_ext = super::constructor::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");
    }

    #[test]
    fn test_read_file_content() {
        // Make up a file name.
        let filename = format!("/tmp/{}_file.md", generate_random_friendly_id());
        println!("🍍🍎🍏filename: {}", filename);

        // Write some content to this file.
        let content = "This is a test.\nThis is only a test.";
        std::fs::write(filename.clone(), content).unwrap();

        let content = super::constructor::get_content(&Some(filename.clone()));
        assert_eq!(content.len(), 2);

        // Delete the file.
        std::fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_state_constructor() {
        // Make up a file name.
        let filename = format!("/tmp/{}_file.md", generate_random_friendly_id());
        let maybe_file_path = Some(filename.clone());
        println!("🍍🍎🍏filename: {}", filename);

        // Write some content to this file.
        let content = "This is a test.\nThis is only a test.";
        std::fs::write(filename.clone(), content).unwrap();

        // Create a state.
        let state = super::constructor::new(&maybe_file_path);

        // Check the state.
        assert_eq!(state.editor_buffers.len(), 1);
        assert_eq!(state.dialog_buffers.len(), 0);
        assert_eq!(
            state
                .editor_buffers
                .contains_key(&FlexBoxId::from(Id::Editor)),
            true
        );
        assert_eq!(
            state
                .editor_buffers
                .get(&FlexBoxId::from(Id::Editor))
                .unwrap()
                .editor_content
                .lines
                .len(),
            2
        );
        assert_eq!(
            state
                .editor_buffers
                .get(&FlexBoxId::from(Id::Editor))
                .unwrap()
                .editor_content
                .lines
                .iter()
                .map(|us| us.string.clone())
                .collect::<Vec<String>>()
                .join("\n"),
            content
        );

        // Delete the file.
        std::fs::remove_file(filename).unwrap();
    }
}

pub mod constructor {
    use std::{ffi::OsStr, path::Path};

    use super::*;

    impl Default for State {
        fn default() -> Self {
            Self {
                editor_buffers: create_hash_map_of_editor_buffers(&None),
                dialog_buffers: Default::default(),
            }
        }
    }

    pub fn new(maybe_file_path: &Option<String>) -> State {
        match maybe_file_path {
            Some(_) => State {
                editor_buffers: create_hash_map_of_editor_buffers(&maybe_file_path),
                dialog_buffers: Default::default(),
            },
            None => State::default(),
        }
    }

    fn create_hash_map_of_editor_buffers(
        maybe_file_path: &Option<String>,
    ) -> HashMap<FlexBoxId, EditorBuffer> {
        let editor_buffer = {
            let mut editor_buffer =
                EditorBuffer::new_empty(Some(get_file_extension(&maybe_file_path)));
            editor_buffer.set_lines(get_content(&maybe_file_path));
            editor_buffer
        };

        let hash_map = {
            let mut it = HashMap::new();
            it.insert(FlexBoxId::from(Id::Editor), editor_buffer);
            it
        };

        hash_map
    }

    pub fn get_file_extension(maybe_file_path: &Option<String>) -> String {
        if let Some(file_path) = maybe_file_path {
            let maybe_extension =
                Path::new(file_path).extension().and_then(OsStr::to_str);
            if let Some(extension) = maybe_extension {
                if extension.is_empty() {
                    return DEFAULT_SYN_HI_FILE_EXT.to_owned();
                }
                return extension.to_owned();
            }
        }

        return DEFAULT_SYN_HI_FILE_EXT.to_owned();
    }

    pub fn get_content(maybe_file_path: &Option<String>) -> Vec<String> {
        // Get the content if the file exists, and it can be read.
        if let Some(file_path) = maybe_file_path {
            let it = std::fs::read_to_string(file_path);
            if let Ok(it) = it {
                return it.lines().map(|s| s.to_string()).collect();
            }
        }
        // Otherwise, an empty vec is returned.
        vec![]
    }
}

mod impl_editor_support {
    use super::*;

    impl HasEditorBuffers for State {
        fn get_mut_editor_buffer(&mut self, id: FlexBoxId) -> Option<&mut EditorBuffer> {
            if let Some(buffer) = self.editor_buffers.get_mut(&id) {
                Some(buffer)
            } else {
                None
            }
        }

        fn insert_editor_buffer(&mut self, id: FlexBoxId, buffer: EditorBuffer) {
            self.editor_buffers.insert(id, buffer);
        }

        fn contains_editor_buffer(&self, id: FlexBoxId) -> bool {
            self.editor_buffers.contains_key(&id)
        }
    }
}

mod impl_dialog_support {
    use super::*;

    impl HasDialogBuffers for State {
        fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer> {
            self.dialog_buffers.get_mut(&id)
        }
    }
}

mod impl_debug_format {
    use super::*;

    impl Display for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt(self, f) }
    }

    impl Debug for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt(self, f) }
    }

    fn fmt(this: &State, f: &mut Formatter<'_>) -> Result {
        write! { f,
            "\nState [\n\
            - dialog_buffers:\n{:?}\n\
            - editor_buffers:\n{:?}\n\
            ]",
            this.dialog_buffers,
            this.editor_buffers,
        }
    }
}
