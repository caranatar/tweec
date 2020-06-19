//! Handles the actual running of the compiler

use crate::linter;
use crate::utils;
use crate::Config;
use crate::StoryFormat;

use tweep::Story;

use clap::{crate_name, crate_version};

use color_eyre::Result;
use eyre::WrapErr;

use horrorshow::html;

use std::fs::File;
use std::io::Write;

use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

/// Runs the compiler
pub fn run() -> Result<()> {
    let config = Config::build()?;

    let mut stdout = StandardStream::stdout(config.use_color);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;

    let story = linter::lint(Story::from_paths(&config.inputs), &config, &mut stdout)?;

    if config.linting {
        std::process::exit(0);
    }

    let story_format = StoryFormat::parse(&config.format_file).wrap_err_with(|| {
        format!(
            "Failed to parse story format file: {:?}",
            &config.format_file
        )
    })?;
    let story_title = story.title.as_deref().unwrap_or("Untitled Story");
    let story_data = format!(
        "{}",
        html! {
            tw-storydata(name = story_title,
                         startnode = utils::get_start_passage_pid(&story).unwrap(),
                         creator = crate_name!(),
                         creator-version = crate_version!(),
                         ifid = story.data.as_ref().unwrap().ifid.as_str(),
                         zoom = story.data.as_ref().unwrap().zoom.unwrap_or(1.),
                         format = story_format.name.as_str(),
                         format-version = story_format.version.as_str(),
                         options = "",
                         hidden = "") {
                style(id = "twine-user-stylesheet",
                      type = "text_twine-css",
                      role = "stylesheet") {
                    : story.stylesheets.join("\n")
                }

                script(id = "twine-user-script",
                       type = "text/twine-javascript",
                       role = "script") {
                    : story.scripts.join("\n")
                }

                @ for (name,passage) in story.passages.iter() {
                    tw-passagedata(name = name,
                                   pid = utils::get_pid(passage),
                                   tags = passage.header.tags.join(" "),
                                   position = passage
                                     .header
                                     .metadata["position"]
                                     .as_str()
                                     .unwrap(),
                                   size = passage
                                     .header
                                     .metadata["size"]
                                     .as_str()
                                     .unwrap()) {
                        : utils::get_content(passage)
                    }
                }
            }
        }
    );

    let output = story_format
        .source
        .replace("{{STORY_NAME}}", story_title)
        .replace("{{STORY_DATA}}", &story_data);
    let file_name = config
        .output_file
        .unwrap_or(format!("{}.html", story_title));
    let mut file = File::create(&file_name).ok().unwrap();
    writeln!(file, "{}", output)
        .wrap_err_with(|| format!("Failed to write output file {}", &file_name))?;

    if config.should_open {
        opener::open(&file_name)
            .wrap_err_with(|| format!("Failed to open output file {}", &file_name))?;
    }

    std::process::exit(0);
}
