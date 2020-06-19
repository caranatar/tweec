//! Tweec is a compiler for the Twee v3 interactive fiction format
//!
//! Tweec is in a pre-alpha state and currently lacks several of its planned
//! features.
//!
//! - [ ] IFID generation
//! - [ ] StoryData story format detection
//! - [ ] Decompilation of Twine2 HTML
//!
//! Some nice-to-haves that I may eventually work on:
//! - [ ] LSP integration
//! - [ ] Plugin system for linting specific story formats
//! - [ ] File/directory watcher
pub type StoryResult = std::result::Result<tweep::Story, tweep::ContextErrorList>;

mod config;
pub use config::CliConfig;
pub use config::Config;
pub use config::ConfigFile;

pub mod issue;
pub use issue::Issue;

mod story_files;
pub use story_files::StoryFiles;

mod story_format;
pub use story_format::StoryFormat;

pub mod utils;

pub mod linter;

pub mod tweec;
