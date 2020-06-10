use serde::{Deserialize, Serialize};

use color_eyre::Result;
use eyre::{eyre, WrapErr};

use std::fs::File;
use std::io::Read;

fn default_name() -> String {
    "Untitled Story Format".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoryFormat {
    // name: (string) Optional. The name of the story format. (Omitting the name
    // will lead to an Untitled Story Format.)
    #[serde(default = "default_name")]
    pub name: String,

    // version: (string) Required, and semantic version-style formatting
    // (x.y.z, e.g., 1.2.1) of the version is also required.
    pub version: String,

    // author: (string) Optional.
    pub author: Option<String>,

    // description: (string) Optional.
    pub description: Option<String>,

    // image: (string) Optional. The filename of an image (ideally SVG) served
    // from the same directory as the format.js file.
    pub image: Option<String>,

    // url: (string) Optional. The URL of the directory containing the format.js
    // file.
    pub url: Option<String>,

    // license: (string) Optional.
    pub license: Option<String>,

    // proofing: (boolean) Optional (defaults to false). True if the story
    // format is a "proofing" format. The distinction is relevant only in the
    // Twine 2 UI.
    #[serde(default)]
    pub proofing: bool,

    // source: (string) Required. An adequately escaped string containing the
    // full HTML output of the story format, including the two placeholders
    // {{STORY_NAME}} and {{STORY_DATA}}. (The placeholders are not themselves
    // required.)
    pub source: String,
}

impl StoryFormat {
    pub fn parse(file_path: &std::path::PathBuf) -> Result<StoryFormat> {
        let mut format_file = File::open(file_path)?;

        let mut contents = String::new();
        format_file.read_to_string(&mut contents)?;

        let start = contents
            .find('{')
            .ok_or_else(|| eyre!("Could not find Twine2 JSON blob"))?;
        let end = if contents.contains("harlowe") {
            contents.rfind(",\"setup\":")
        } else {
            contents.rfind('}')
        }
        .ok_or_else(|| eyre!("Could not find Twine2 JSON blob"))?;

        let mut json_blob_contents = contents[start..end].to_owned();
        json_blob_contents.push('}');

        let f = serde_json::from_str(&json_blob_contents)
            .wrap_err_with(|| "Failed to parse story format JSON")?;
        Ok(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let input = r#"{ "version": "1.2.3", "source": "blah" }"#;
        let res: serde_json::Result<StoryFormat> = serde_json::from_str(input);
        assert!(res.is_ok());
        let story_format = res.ok().unwrap();
        assert_eq!(story_format.name, "Untitled Story Format");
        assert_eq!(story_format.version, "1.2.3");
        assert_eq!(story_format.author, None);
        assert_eq!(story_format.description, None);
        assert_eq!(story_format.image, None);
        assert_eq!(story_format.url, None);
        assert_eq!(story_format.license, None);
        assert_eq!(story_format.proofing, false);
        assert_eq!(story_format.source, "blah");
    }
}
