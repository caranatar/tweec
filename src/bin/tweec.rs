use tweec::Config;
use tweec::Issue;
use tweec::StoryFiles;
use tweec::StoryFormat;
use tweec::StoryResult;
use tweep::Output;
use tweep::Story;
use tweep::TwinePassage;
use tweep::Warning;

use clap::{crate_name, crate_version};

use color_eyre::Result;
use eyre::{eyre, WrapErr};

use horrorshow::html;

use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;

use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use codespan_reporting::term;

fn get_start_passage_pid(story: &Story) -> Option<usize> {
    let start_name = story.get_start_passage_name().expect("No start passage");
    let passage = &story.passages.get(start_name);
    passage.and_then(|twine| Some(twine.content.pid))
}

fn get_pid(twine: &TwinePassage) -> usize {
    twine.content.pid
}

fn get_content(twine: &TwinePassage) -> &str {
    twine.content.content.as_str()
}

fn filter_and_sort_issues(
    story_result: &StoryResult,
    mut warnings: Vec<Warning>,
    config: &Config,
) -> (Vec<Issue>, bool) {
    let mut issues = Vec::new();
    let mut is_err = false;

    let all = "all".to_string();
    let allow_all = config.allowed.contains(&all);
    let deny_all = config.denied.contains(&all);
    for warning in warnings.drain(..) {
        let name = warning.get_name().to_string();
        if allow_all || config.allowed.contains(&name) {
            continue;
        }
        let denied = deny_all || config.denied.contains(&name);
        if denied {
            is_err = true;
        }
        issues.push(Issue::Warning { warning, denied });
    }

    if let Err(e) = &story_result {
        is_err = true;
        for e in &e.error_list.errors {
            issues.push(Issue::Error(e.clone()));
        }
    }

    issues.sort_by(|left, right| {
        let left = match left {
            Issue::Error(e) => &e.context,
            Issue::Warning { warning, .. } => &warning.context,
        };
        let right = match right {
            Issue::Error(e) => &e.context,
            Issue::Warning { warning, .. } => &warning.context,
        };
        match (left, right) {
            (None, _) => Ordering::Less,
            (_, None) => Ordering::Greater,
            (Some(lctx), Some(rctx)) => match (lctx.get_file_name(), rctx.get_file_name()) {
                (None, _) => Ordering::Less,
                (_, None) => Ordering::Greater,
                (Some(_), Some(_)) => {
                    let lpos = lctx.get_start_position();
                    let rpos = rctx.get_start_position();
                    let (lline, lcol) = (lpos.line, lpos.column);
                    let (rline, rcol) = (rpos.line, rpos.column);

                    if lline == rline {
                        lcol.cmp(&rcol)
                    } else {
                        lline.cmp(&rline)
                    }
                }
            },
        }
    });

    (issues, is_err)
}

fn print_issue(issue: &Issue, stdout: &mut StandardStream) -> Result<()> {
    let kind = match issue {
        Issue::Error(_) | Issue::Warning { denied: true, .. } => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            "Error"
        }
        Issue::Warning { denied: false, .. } => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
            "Warning"
        }
    };
    write!(stdout, "{}: ", kind)?;
    stdout.reset()?;
    writeln!(
        stdout,
        "{}",
        match issue {
            Issue::Error(e) => format!("{}", e),
            Issue::Warning { warning, .. } => format!("{}", warning),
        }
    )?;
    Ok(())
}

fn lint(
    story_output: Output<StoryResult>,
    config: &Config,
    stdout: &mut StandardStream,
) -> Result<Story> {
    let (story_result, warnings) = story_output.take();

    let story_files = StoryFiles::new(&story_result);

    let (issues, is_err) = filter_and_sort_issues(&story_result, warnings, config);

    if config.compact {
        for issue in &issues {
            print_issue(issue, stdout)?;
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

fn main() -> Result<()> {
    let config = Config::build()?;

    let mut stdout = StandardStream::stdout(config.use_color);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;

    let story = lint(Story::from_paths(&config.inputs), &config, &mut stdout)?;

    if config.linting {
        std::process::exit(0);
    }

    let story_format = StoryFormat::parse(&config.format_file)
        .wrap_err_with(|| format!("Failed to parse story format file: {:?}", &config.format_file))?;
    let story_title = story
        .title
        .as_deref()
        .unwrap_or("Untitled Story");
    let story_data = format!(
        "{}",
        html! {
            tw-storydata(name = story_title,
                         startnode = get_start_passage_pid(&story).unwrap(),
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
                                   pid = get_pid(passage),
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
                        : get_content(passage)
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
        if let Err(e) = opener::open(&file_name) {
            println!("Couldn't open output file {}: {}", &file_name, e);
        }
    }

    std::process::exit(0);
}
