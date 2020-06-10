use crate::StoryFiles;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::Files;
use std::cmp::Ordering;
use std::ops::Range;
use tweep::FullContext;
use tweep::Warning;
use tweep::WarningKind;

pub enum Issue {
    Error(tweep::Error),
    Warning { warning: Warning, denied: bool },
}

impl Issue {
    fn get_name(&self) -> &str {
        match self {
            Issue::Error(e) => e.get_name(),
            Issue::Warning { warning: w, .. } => w.kind.get_name(),
        }
    }

    fn get_message(&self) -> String {
        match self {
            Issue::Error(e) => format!("{}", e.kind),
            Issue::Warning { warning, .. } => format!("{}", warning.kind),
        }
    }

    fn get_referent(&self) -> Option<&FullContext> {
        match self {
            Issue::Error(_) => None,
            Issue::Warning { warning, .. } => warning.get_referent(),
        }
    }

    fn get_referent_file_id_and_range(
        &self,
        story_files: &StoryFiles,
    ) -> Option<(usize, Range<usize>)> {
        self.get_referent().and_then(|context| {
            context
                .get_file_name()
                .as_ref()
                .and_then(|file_name| story_files.code_map.lookup_id(file_name.clone()))
                .map(|id| (id, context.get_byte_range()))
        })
    }

    fn get_file_id_and_range(&self, story_files: &StoryFiles) -> Option<(usize, Range<usize>)> {
        let context = match self {
            Issue::Error(e) => &e.context,
            Issue::Warning { warning, .. } => &warning.context,
        };
        context.as_ref().and_then(|context| {
            context
                .get_file_name()
                .as_ref()
                .and_then(|file_name| story_files.code_map.lookup_id(file_name.clone()))
                .map(|id| (id, context.get_byte_range()))
        })
    }

    pub fn report(&self, story_files: &StoryFiles) -> Diagnostic<<StoryFiles as Files>::FileId> {
        let diagnostic = match self {
            Issue::Error(_) | Issue::Warning { denied: true, .. } => Diagnostic::error(),
            Issue::Warning { denied: false, .. } => Diagnostic::warning(),
        }
        .with_message(self.get_message())
        .with_code(self.get_name());

        let help_message = match self {
            Issue::Warning { warning: w, .. } => match &w.kind {
                WarningKind::DeadLink(dead) => {
                    story_files.passage_names.as_ref().and_then(|names| {
                        did_you_mean(dead, names).pop().map(|suggestion| {
                            format!("Found passage with similar name: \"{}\"", suggestion)
                        })
                    })
                }
                WarningKind::WhitespaceInLink => w.context.as_ref().and_then(|ctx| {
                    // Get the full link
                    let link = ctx.get_contents();

                    // Pull out the [[contents]]
                    let contents = &link[2..link.len() - 2];

                    // Get the target of the link
                    let target = if contents.contains('|') {
                        let mut iter = contents.split('|');
                        let _ = iter.next();
                        iter.next().unwrap()
                    } else if contents.contains("<-") {
                        contents.split("<-").next().unwrap()
                    } else if contents.contains("->") {
                        let mut iter = contents.split("->");
                        let _ = iter.next();
                        iter.next().unwrap()
                    } else {
                        contents
                    };

                    // Trim the target and create a valid link
                    let trimmed = target.trim();
                    let suggested = link.replace(target, trimmed);
                    Some(format!("Try replacing {} with {}", link, suggested))
                }),
                _ => None,
            },
            _ => None,
        };

        self.get_file_id_and_range(&story_files)
            .and_then(|(fid, range)| {
                let mut labels = Vec::new();
                labels.push(Label::primary(fid, range));

                self.get_referent_file_id_and_range(&story_files)
                    .and_then(|(fid, range)| {
                        labels.push(
                            Label::secondary(fid, range)
                                .with_message("Previously defined here. Duplicate discarded."),
                        );

                        Some(())
                    });

                let mut notes = Vec::new();
                if let Some(msg) = help_message {
                    notes.push(msg);
                }

                Some(diagnostic.clone().with_labels(labels).with_notes(notes))
            })
            .unwrap_or(diagnostic)
    }
}

fn did_you_mean<T, I>(v: &str, possible_values: I) -> Vec<String>
where
    T: AsRef<str>,
    I: IntoIterator<Item = T>,
{
    let mut candidates: Vec<(f64, String)> = possible_values
        .into_iter()
        .map(|pv| (strsim::jaro_winkler(v, pv.as_ref()), pv.as_ref().to_owned()))
        .filter(|(confidence, _)| *confidence > 0.8)
        .collect();
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    candidates.into_iter().map(|(_, pv)| pv).collect()
}
