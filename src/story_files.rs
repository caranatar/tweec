use crate::StoryResult;
use codespan_reporting::files::Files;
use std::ops::Range;
use tweep::CodeMap;

/// Adapts a `Story` and its `CodeMap` for use by codespan
pub struct StoryFiles<'a> {
    /// The code map
    pub code_map: &'a CodeMap,

    /// An optional list of passage names. If the story has errors this is None
    pub passage_names: Option<Vec<String>>,
}

impl<'a> StoryFiles<'a> {
    /// Creats a new instance from the given `StoryResult`
    pub fn new(res: &'a StoryResult) -> Self {
        let (code_map, passage_names) = match res {
            Ok(story) => {
                let names = story.passages.keys().cloned().collect();
                (&story.code_map, Some(names))
            }
            Err(e) => {
                println!("{:?}", &e.code_map);
                (&e.code_map, None)
            }
        };

        StoryFiles {
            code_map,
            passage_names,
        }
    }
}

impl<'a> Files<'a> for StoryFiles<'a> {
    type FileId = usize;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, id: Self::FileId) -> Option<Self::Name> {
        self.code_map.lookup_name(id)
    }

    fn source(&'a self, id: Self::FileId) -> Option<Self::Source> {
        self.code_map
            .get_context(id)
            .map(|context| context.get_contents())
    }

    fn line_index(&'a self, id: Self::FileId, byte_index: usize) -> Option<usize> {
        self.code_map.line_starts(id).and_then(|bytes| {
            bytes
                .binary_search(&byte_index)
                .or_else(|idx: usize| -> Result<usize, usize> { Ok(idx - 1) })
                .ok()
        })
    }

    fn line_range(&'a self, id: Self::FileId, line_index: usize) -> Option<Range<usize>> {
        self.code_map.line_range(id, line_index + 1)
    }
}
