use clap::{crate_authors, crate_description, crate_name, crate_version};
use clap::{App, Arg};
use color_eyre::Result;
use eyre::eyre;
use eyre::WrapErr;
use json_comments::StripComments;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use termcolor::ColorChoice;

use std::path::PathBuf;

pub struct Config {
    pub linting: bool,
    pub inputs: Vec<String>,
    pub format_file: PathBuf,
    pub output_file: Option<String>,
    pub should_open: bool,
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
    pub use_color: ColorChoice,
    pub compact: bool,
}

impl Config {
    pub fn build() -> Result<Self> {
        let config_file = ConfigFile::load()?;
        let cli_config = CliConfig::from_args();
        Ok(Config::layer(config_file, cli_config))
    }

    pub fn layer(config_file: ConfigFile, cli_config: CliConfig) -> Self {
        let format_file = cli_config
            .format
            .as_ref()
            .map(|f| {
                config_file
                    .formats
                    .get(f)
                    .cloned()
                    .unwrap_or_else(|| f.into())
            })
            .unwrap_or_else(|| "format.js".into());

        let mut allowed = cli_config.allowed;
        let mut default_allowed = config_file
            .format_configs
            .get("default")
            .map(|f| f.allow.clone())
            .unwrap_or_default();
        allowed.append(&mut default_allowed);
        let mut format_allowed = cli_config
            .format
            .as_ref()
            .map(|f| {
                config_file
                    .format_configs
                    .get(f)
                    .map(|f| f.allow.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        allowed.append(&mut format_allowed);

        let mut denied = cli_config.denied;
        let mut default_denied = config_file
            .format_configs
            .get("default")
            .map(|f| f.deny.clone())
            .unwrap_or_default();
        denied.append(&mut default_denied);
        let mut format_denied = cli_config
            .format
            .as_ref()
            .map(|f| {
                config_file
                    .format_configs
                    .get(f)
                    .map(|f| f.deny.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        denied.append(&mut format_denied);

        Config {
            linting: cli_config.linting,
            inputs: cli_config.inputs,
            format_file,
            output_file: cli_config.output_file,
            should_open: cli_config.should_open,
            allowed,
            denied,
            use_color: cli_config.use_color,
            compact: cli_config.compact,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FormatConfig {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFileInternal {
    pub format_paths: Vec<String>,
    pub format_configs: HashMap<String, FormatConfig>,
}

#[derive(Debug)]
pub struct ConfigFile {
    pub formats: HashMap<String, std::path::PathBuf>,
    pub format_configs: HashMap<String, FormatConfig>,
}

impl ConfigFile {
    pub fn load() -> Result<Self> {
        let config_path = dirs_next::config_dir()
            .ok_or_else(|| eyre!("Error getting config directory"))?
            .join("tweec/config.json");

        let config_contents = if !config_path.exists() {
            let prefix = config_path.parent().unwrap();
            std::fs::create_dir_all(prefix)
                .wrap_err_with(|| format!("Error creating config directory: {:?}", prefix))?;
            let default_config = r#"// This file defines the configuration for tweec
// It is mostly standard JSON, but supports //, /**/, and # style comments.
//
// For path related configuration, tweec defines several special variables that
// can be used to specify locations:
// * $TWEEC_BIN_DIR: directory in which the tweec executable is located
// * $TWEEC_DATA_DIR: tweec's system data dir (OS-specific; see documentation)
// * $PWD: directory from which tweec is being invoked
// * $HOME: user's home directory (~ is not currently supported)
//
// Arbitrary environment variables are not currently supported
{
  // Directories to search for story formats in
  "format_paths": [
    "$TWEEC_DATA_DIR/storyformats",
    "$TWEEC_DATA_DIR/.storyformats",
    "$TWEEC_BIN_DIR/storyformats",
    "$TWEEC_BIN_DIR/.storyformats",
    "$HOME/storyformats",
    "$HOME/.storyformats",
    "$PWD/storyformats",
    "$PWD/.storyformats"
  ],
  "format_configs": {
    // This is the default configuration which other configurations will be
    // layered over. Config items defined in story format-specific config chunks
    // will be appended to the values given in default, but items not defined
    // will use the default config's value.
    "default": {
      // Warnings to ignore ("allow")
      "allow": [],
      // Warnings to treat as errors ("deny")
      "deny": []
    },
    "sugarcube-2": {
      // SugarCube handles whitespace in links, so allow them when using it
      "allow": [ "WhitespaceInLink" ]
    }
  }
}"#;
            let mut config_file = File::create(config_path)?;
            config_file.write_all(default_config.as_bytes())?;

            default_config.to_string()
        } else {
            use std::io::Read;
            let mut config_file = File::open(config_path)?;
            let mut contents: String = String::new();
            config_file.read_to_string(&mut contents)?;
            contents
        };
        // Strip the comments from the input (use `as_bytes()` to get a `Read`).
        let stripped = StripComments::new(config_contents.as_bytes());
        // Parse the string of data into serde_json::Value.
        let cf: ConfigFileInternal = serde_json::from_reader(stripped)?;
        println!("{:?}", cf);

        type T = color_eyre::Result<HashMap<String, PathBuf>>;
        let formats = cf
            .format_paths
            .iter()
            .fold(Ok(HashMap::new()), |acc: T, p| {
                let mut acc = acc?;
                let mut path = p.clone();
                while let Some(start) = path.find('$') {
                    let end = match path[start..].find('/') {
                        Some(pos) => pos,
                        None => path.len(),
                    };
                    let var = &path[start..end];
                    let var_name = &var[1..];
                    let replace = match var_name {
                        "HOME" => dirs_next::home_dir().ok_or_else(|| eyre!("Failed to get HOME")),
                        "PWD" => std::env::current_dir().wrap_err_with(|| "Failed to get PWD"),
                        "TWEEC_BIN_DIR" => match std::env::current_exe() {
                            Ok(ok) => ok
                                .parent()
                                .map(|p| p.to_path_buf())
                                .ok_or_else(|| eyre!("Failed to get tweec executable's parent")),
                            Err(err) => Err(err).wrap_err_with(|| "Failed to get TWEEC_BIN_DIR"),
                        },
                        "TWEEC_DATA_DIR" => dirs_next::data_dir()
                            .ok_or_else(|| eyre!("Failed to get TWEEC_DATA_DIR")),
                        _ => Err(eyre!(
                            "Arbitrary environment variables are not currently supported"
                        )),
                    }
                    .map(|p| p.into_os_string().to_string_lossy().into_owned())
                    .wrap_err_with(|| format!("Error while parsing {}", p))?;
                    path = path.replace(var, &replace);
                }

                let path_buf: PathBuf = path.clone().into();
                if !path_buf.exists() {
                    // Continue
                    return Ok(acc);
                }

                if !path_buf.is_dir() {
                    return Err(eyre!("Path {} is not a directory", path));
                }

                let formats_dir = std::fs::read_dir(path_buf)
                    .wrap_err_with(|| format!("Error while reading directory {}", path))?;
                for entry in formats_dir {
                    if entry.is_err() {
                        continue;
                    }

                    let format_path = entry.ok().unwrap().path();
                    if !format_path.is_dir() {
                        continue;
                    }

                    let dir = std::fs::read_dir(format_path.clone());
                    let dir = match dir {
                        Ok(dir) => dir,
                        Err(_) => continue,
                    };

                    for entry in dir {
                        let entry = match entry {
                            Ok(entry) => entry,
                            Err(_) => continue,
                        };
                        if entry.file_name() == "format.js" {
                            let dir_name = format_path.file_name().ok_or_else(|| {
                                eyre!("Error getting directory name for path {}", path)
                            })?;
                            let dir_name = dir_name.to_string_lossy().into_owned();
                            acc.entry(dir_name).or_insert_with(|| entry.path());
                        }
                    }
                }

                Ok(acc)
            })?;

        println!("formats: {:?}", formats);

        Ok(ConfigFile {
            formats,
            format_configs: cf.format_configs,
        })
    }
}

pub struct CliConfig {
    pub linting: bool,
    pub inputs: Vec<String>,
    pub format: Option<String>,
    pub output_file: Option<String>,
    pub should_open: bool,
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
    pub use_color: ColorChoice,
    pub compact: bool,
}

impl CliConfig {
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
                    .help("Sets the story format by name (e.g., sugarcube-2) or file location")
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
        let format = m.value_of("format").map(|s| s.to_string());
        let output_file = m.value_of("output").map(|s| s.to_string());
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

        CliConfig {
            linting,
            inputs,
            format,
            output_file,
            should_open,
            allowed,
            denied,
            use_color,
            compact,
        }
    }
}
