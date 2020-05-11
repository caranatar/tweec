use tweec::Config;
use tweec::StoryFormat;

use tweep::ErrorList;
use tweep::Output;
use tweep::Passage;
use tweep::PassageContent;
use tweep::Story;

use clap::{crate_name, crate_version};

use color_eyre::Result;
use eyre::{eyre, WrapErr};

use horrorshow::html;

use std::fs::File;
use std::io::Write;

use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

enum Issue {
    Error(tweep::Error),
    Warning {
        warning: tweep::Warning,
        denied: bool,
    },
}

fn get_start_passage_pid(story: &Story) -> Option<usize> {
    let start_name = story.get_start_passage_name().expect("No start passage");
    let passage = &story.passages.get(start_name);
    passage.and_then(|p| {
        if let PassageContent::Normal(twine) = &p.content {
            Some(twine.pid)
        } else {
            None
        }
    })
}

fn get_pid(passage: &Passage) -> usize {
    if let PassageContent::Normal(twine) = &passage.content {
        twine.pid
    } else {
        panic!("Expected Twine Content");
    }
}

fn get_content(passage: &Passage) -> &str {
    if let PassageContent::Normal(twine) = &passage.content {
        twine.content.as_str()
    } else {
        panic!("Expected Twine Content");
    }
}

fn lint(
    story_output: Output<std::result::Result<Story, ErrorList>>,
    config: &Config,
    stdout: &mut StandardStream,
) -> Result<Story> {
    let mut is_err = false;
    let mut issues = Vec::new();

    let (story_result, mut warnings) = story_output.take();
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
        for e in &e.errors {
            issues.push(Issue::Error(e.clone()));
        }
    }

    issues.sort_by(|left, right| {
        use std::cmp::Ordering;
        use tweep::{Position, Positional};
        let left = match left {
            Issue::Error(e) => e.get_position(),
            Issue::Warning { warning, .. } => warning.get_position(),
        };
        let right = match right {
            Issue::Error(e) => e.get_position(),
            Issue::Warning { warning, .. } => warning.get_position(),
        };

        match (left, right) {
            (Position::StoryLevel, _) => Ordering::Less,
            (Position::File(lf, lr, lc), Position::File(rf, rr, rc)) => {
                if lf == rf {
                    if lr == rr {
                        lc.cmp(rc)
                    } else {
                        lr.cmp(rr)
                    }
                } else {
                    lf.cmp(rf)
                }
            }
            _ => panic!("Bug: Unexpected position types: {:?}, {:?}", left, right),
        }
    });

    for issue in issues {
        let kind = match issue {
            Issue::Error(_) | Issue::Warning { denied: true, .. } => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                "Error"
            },
            Issue::Warning { denied: false, .. } => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
                "Warning"
            },
        };
        write!(
            stdout,
            "{}: ",
            kind)?;
        stdout.reset()?;
        writeln!(
            stdout,
            "{}",
            match issue {
                Issue::Error(e) => format!("{}", e),
                Issue::Warning { warning, .. } => format!("{}", warning),
            }
        )?;
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
    let config = Config::from_args();

    let mut stdout = StandardStream::stdout(config.use_color);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;

    let story = lint(Story::from_paths(&config.inputs), &config, &mut stdout)?;

    if config.linting {
        std::process::exit(0);
    }

    let story_format = StoryFormat::parse(&config.format_file)
        .wrap_err_with(|| format!("Failed to parse story format file: {}", &config.format_file))?;
    let story_title = story
        .title
        .as_ref()
        .and_then(|x| Some(x.as_str()))
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

    let output = story_format.source.replace("{{STORY_NAME}}", story_title).replace("{{STORY_DATA}}", &story_data);
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
