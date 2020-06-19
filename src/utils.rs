//! Utility functions for dealing with tweep types
use tweep::Story;
use tweep::TwinePassage;

/// Gets the pid of the start passage of a story, if possible
pub fn get_start_passage_pid(story: &Story) -> Option<usize> {
    let start_name = story.get_start_passage_name().expect("No start passage");
    let passage = &story.passages.get(start_name);
    passage.and_then(|twine| Some(twine.content.pid))
}

/// Gets the pid of a `TwinePassage`
pub fn get_pid(twine: &TwinePassage) -> usize {
    twine.content.pid
}

/// Gets the contents of a `TwinePassage`
pub fn get_content(twine: &TwinePassage) -> &str {
    twine.content.content.as_str()
}
