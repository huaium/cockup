use super::{copy::copy, paths::magic, state::CopyState};
use crate::{
    config::{Rule, Symlinks},
    report,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn rule(
    rule: &Rule,
    destination: &Path,
    restore: bool,
    metadata: bool,
    symlinks: Symlinks,
    state: &mut CopyState,
) -> usize {
    let mut base = rule.src.clone();
    while magic(&base) {
        base = base.parent().unwrap().to_path_buf();
    }
    let prefix = rule.src.strip_prefix(&base).unwrap();
    let backup = destination.join(&rule.to);
    let (from, to) = if restore {
        (&backup, &base)
    } else {
        (&base, &backup)
    };
    let mut failures = 0;
    for target in &rule.targets {
        let target = prefix.join(target);
        let source = from.join(target);
        let paths: Result<Vec<PathBuf>, String> = if fs::symlink_metadata(&source).is_ok() {
            Ok(vec![source.clone()])
        } else if magic(&source) {
            glob::glob_with(
                &source.to_string_lossy(),
                glob::MatchOptions {
                    case_sensitive: true,
                    require_literal_separator: true,
                    require_literal_leading_dot: false,
                },
            )
            .map_err(|e| e.to_string())
            .and_then(|paths| {
                paths
                    .filter(|p| {
                        p.as_ref().map_or(true, |p| {
                            let last = p
                                .as_os_str()
                                .as_encoded_bytes()
                                .rsplit(|c| *c == b'/')
                                .next()
                                .unwrap_or_default();
                            last != b"."
                                && last != b".."
                                && glob::Pattern::new(&source.to_string_lossy()).is_ok_and(
                                    |pattern| {
                                        pattern.matches_path_with(
                                            p,
                                            glob::MatchOptions {
                                                case_sensitive: true,
                                                require_literal_separator: true,
                                                require_literal_leading_dot: true,
                                            },
                                        )
                                    },
                                )
                        })
                    })
                    .map(|p| p.map_err(|e| e.to_string()))
                    .collect()
            })
        } else {
            report::warning(&format!("Source not found, skipping: {}", source.display()));
            Ok(vec![])
        };
        match paths {
            Err(e) => {
                report::error(&e);
                failures += 1;
            }
            Ok(paths) => {
                if !state.planning && magic(&source) && !paths.is_empty() {
                    report::success(&format!(
                        "Target pattern matched ({} found): {}",
                        paths.len(),
                        report::display_path(&source)
                    ));
                }
                if paths.is_empty() && magic(&source) {
                    report::error(&format!(
                        "Matches not found for pattern: {}",
                        source.display()
                    ));
                    failures += 1;
                }
                for src in paths {
                    let relative = src.strip_prefix(from).unwrap();
                    if let Err(e) = copy(
                        &src,
                        &if restore {
                            state.mapped(&to.join(relative))
                        } else {
                            to.join(relative)
                        },
                        metadata,
                        symlinks,
                        &[],
                        state,
                    ) {
                        report::error(&e);
                        failures += 1;
                    }
                }
            }
        }
    }
    failures
}
