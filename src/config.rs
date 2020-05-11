use clap::{crate_authors, crate_description, crate_name, crate_version};
use clap::{App, Arg};

use termcolor::ColorChoice;

pub struct Config {
    pub linting: bool,
    pub inputs: Vec<String>,
    pub format_file: String,
    pub output_file: Option<String>,
    pub should_open: bool,
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
    pub use_color: ColorChoice,
    pub compact: bool,
}

impl Config {
    pub fn from_args() -> Self {
        #[allow(deprecated)]
        let m = App::new(crate_name!())
            .about(crate_description!())
            .author(crate_authors!("\n"))
            .version(crate_version!())
            .arg(
                Arg::with_name("allow")
                    .help("Specifies warnings to ignore. Overrides deny.")
                    .short("a")
                    .long("allow")
                    .takes_value(true)
                    .multiple(true),
            )
            .arg(
                Arg::with_name("color")
                    .help("Turns on colored output")
                    .long("color")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("compact")
                    .help("Turns on compact error and warning output")
                    .long("compact"),
            )
            .arg(
                Arg::with_name("deny")
                    .help("Specifies warnings to treat as errors")
                    .short("D")
                    .long("deny")
                    .takes_value(true)
                    .multiple(true),
            )
            .arg(
                Arg::with_name("format")
                    .help("Sets the location of the story format .js file")
                    .short("f")
                    .long("format")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("lint")
                    .help("Runs the linter without producing any output")
                    .short("L")
                    .long("lint"),
            )
            .arg(
                Arg::with_name("open")
                    .help("Opens the html output in a web browser")
                    .long("open")
                    .conflicts_with("lint"),
            )
            .arg(
                Arg::with_name("output")
                    .help("Sets the output file (default: <Story Title>.html")
                    .short("o")
                    .long("output")
                    .takes_value(true)
                    .conflicts_with("lint"),
            )
            .arg(
                Arg::with_name("INPUT")
                    .help("Sets the input file(s) or directory(s) to use")
                    .required(true)
                    .multiple(true)
                    .index(1),
            )
            .get_matches();

        let linting = m.is_present("lint");
        let inputs: Vec<String> = m
            .values_of("INPUT")
            .unwrap()
            .map(|s| s.to_string())
            .collect();
        let format_file = m.value_of("format").unwrap_or("format.js").to_string();
        let output_file = m.value_of("output").and_then(|s| Some(s.to_string()));
        let should_open = m.is_present("open");
        let allowed = m
            .values_of("allow")
            .unwrap_or_default()
            .map(|s| s.to_string())
            .collect();
        let denied = m
            .values_of("deny")
            .unwrap_or_default()
            .map(|s| s.to_string())
            .collect();
        let use_color = match m.value_of("color").unwrap_or("auto") {
            "always" => ColorChoice::Always,
            "ansi" => ColorChoice::AlwaysAnsi,
            "auto" => {
                if atty::is(atty::Stream::Stdout) {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
            _ => ColorChoice::Never,
        };
        let compact = m.is_present("compact");

        Config {
            linting,
            inputs,
            format_file,
            output_file,
            should_open,
            allowed,
            denied,
            use_color,
            compact,
        }
    }
}
