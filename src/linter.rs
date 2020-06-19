//! Handles linting a story based on the given [`Config`]
//!
//! [`Config`]: struct.Config.html

use crate::issue;
use crate::Config;
use crate::StoryFiles;
use crate::StoryResult;
use codespan_reporting::term;
use color_eyre::Result;
use eyre::eyre;
use std::io::Write;
use termcolor::StandardStream;
use tweep::Output;
use tweep::Story;

/// Lints the given story based on the given config and outputs warnings/errors
/// to the given stream.
///
/// Warnings are ignored or promoted to errors as specified in the config
pub fn lint(
    story_output: Output<StoryResult>,
    config: &Config,
    stdout: &mut StandardStream,
) -> Result<Story> {
    let (story_result, warnings) = story_output.take();

    let story_files = StoryFiles::new(&story_result);

    let (issues, is_err) = issue::filter_and_sort_issues(&story_result, warnings, config);

    if config.compact {
        for issue in &issues {
            issue::print_issue(issue, stdout)?;
        }
    } else {
        let config = term::Config::default();
        for issue in &issues {
            let diagnostic = issue.report(&story_files);
            term::emit(&mut stdout.lock(), &config, &story_files, &diagnostic)?;
        }
    }

    // Force reset of color
    stdout.flush()?;

    if is_err {
        Err(eyre!("Failed due to previous errors"))
    } else {
        Ok(story_result.ok().unwrap())
    }
}
