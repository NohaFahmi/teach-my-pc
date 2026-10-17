use crate::rules::{Action, ActivityRecord, Rule, RuleRepository};
use chrono::Local;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    sync::Mutex,
    thread,
    time::Duration,
};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanPreview {
    pub source_filename: String,
    pub matches_rule: bool,
    pub message: Option<String>,
    pub will_modify_files: bool,
    pub actions: Vec<String>,
    pub errors: Vec<String>,
    pub collisions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleRuntimeStatus {
    pub rule_id: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct WatcherManager {
    statuses: Arc<Mutex<std::collections::HashMap<String, RuleRuntimeStatus>>>,
}

impl WatcherManager {
    pub fn list(&self) -> Result<Vec<RuleRuntimeStatus>, String> {
        let statuses = self
            .statuses
            .lock()
            .map_err(|_| "Watcher status is unavailable".to_owned())?;
        let mut values: Vec<_> = statuses.values().cloned().collect();
        values.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
        Ok(values)
    }
}

pub fn plan(
    rule: &Rule,
    source: &Path,
    folders: &RuleRepository,
) -> Result<Vec<(String, PathBuf)>, String> {
    let source_root = folders.folder_path(&rule.source_folder_id)?;
    let source_real = validate_source(rule, source, folders)?;
    let mut current = source_real.clone();
    let ext = current.extension().map(|s| s.to_os_string());
    let stem = current
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Source filename is not valid UTF-8")?
        .to_owned();
    let date = Local::now().format("%Y-%m-%d").to_string();
    let mut steps = Vec::new();
    for action in &rule.actions {
        let (label, next) = match action {
            Action::Rename { pattern } => {
                let name = pattern
                    .replace("{date}", &date)
                    .replace("{originalName}", &stem);
                if name.is_empty()
                    || name == "."
                    || name == ".."
                    || name.contains(['/', '\\', ':'])
                    || name.chars().any(char::is_control)
                {
                    return Err("Rename produced an invalid filename".into());
                }
                let mut path = current.with_file_name(name);
                if let Some(extension) = &ext {
                    path.set_extension(extension);
                }
                (
                    format!(
                        "Rename to {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ),
                    path,
                )
            }
            Action::Move {
                destination_folder_id,
            }
            | Action::Copy {
                destination_folder_id,
            } => {
                let destination = folders.action_folder(destination_folder_id)?;
                if destination.starts_with(&source_root) || source_root.starts_with(&destination) {
                    return Err("Source and destination folders cannot contain one another".into());
                }
                let path =
                    destination.join(current.file_name().ok_or("Source filename is invalid")?);
                let kind = if matches!(action, Action::Move { .. }) {
                    "Move"
                } else {
                    "Copy"
                };
                (format!("{kind} to {}", path.display()), path)
            }
        };
        if next.exists() {
            return Err(format!(
                "Destination already exists: {}",
                next.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
        steps.push((label, next.clone()));
        current = next;
    }
    Ok(steps)
}

pub fn preview(rule: &Rule, sample: &Path, repo: &RuleRepository) -> Result<PlanPreview, String> {
    let filename = sample
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    validate_source(rule, sample, repo)?;
    if !matches_extension(rule, sample) {
        return Ok(PlanPreview {
            source_filename: filename,
            matches_rule: false,
            message: Some("This file does not match the rule's extension filter.".into()),
            will_modify_files: false,
            actions: vec![],
            errors: vec![],
            collisions: vec![],
        });
    }
    match plan(rule, sample, repo) {
        Ok(steps) => Ok(PlanPreview {
            source_filename: filename,
            matches_rule: true,
            message: None,
            will_modify_files: false,
            actions: steps.into_iter().map(|s| s.0).collect(),
            errors: vec![],
            collisions: vec![],
        }),
        Err(error) => Ok(PlanPreview {
            source_filename: filename,
            matches_rule: true,
            message: None,
            will_modify_files: false,
            actions: vec![],
            collisions: if error.starts_with("Destination already exists") {
                vec![error.clone()]
            } else {
                vec![]
            },
            errors: vec![error],
        }),
    }
}

fn validate_source(
    rule: &Rule,
    source: &Path,
    folders: &RuleRepository,
) -> Result<PathBuf, String> {
    let source_root = folders.folder_path(&rule.source_folder_id)?;
    let source_real = source
        .canonicalize()
        .map_err(|_| "Source file is unavailable")?;
    if !source_real.starts_with(&source_root)
        || fs::symlink_metadata(source)
            .map_err(|_| "Source file is unavailable")?
            .file_type()
            .is_symlink()
        || !source_real.is_file()
    {
        return Err("Source must be a regular file inside its authorized folder".into());
    }
    Ok(source_real)
}

fn matches_extension(rule: &Rule, sample: &Path) -> bool {
    rule.trigger.extensions.is_empty()
        || sample
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                rule.trigger.extensions.iter().any(|wanted| {
                    wanted
                        .trim_start_matches('.')
                        .eq_ignore_ascii_case(extension)
                })
            })
}

pub fn execute(rule: &Rule, source: &Path, repo: &RuleRepository) -> Result<(), String> {
    if !rule.enabled {
        return record_outcome(
            rule,
            source,
            repo,
            vec![],
            None,
            "skipped",
            Some("Rule is disabled".into()),
        );
    }
    let planned = match plan(rule, source, repo) {
        Ok(planned) => planned,
        Err(error) => {
            let status = if error.starts_with("Destination already exists") {
                "skipped"
            } else {
                "failed"
            };
            return record_outcome(rule, source, repo, vec![], None, status, Some(error));
        }
    };
    let mut current = match source.canonicalize() {
        Ok(current) => current,
        Err(_) => {
            return record_outcome(
                rule,
                source,
                repo,
                vec![],
                None,
                "failed",
                Some("Source file is unavailable".into()),
            );
        }
    };
    let mut completed = Vec::new();
    let mut final_error = None;
    for ((label, target), action) in planned.iter().zip(&rule.actions) {
        let result = match action {
            Action::Rename { .. } | Action::Move { .. } => {
                if let (Ok(a), Ok(b)) = (
                    fs::metadata(&current),
                    fs::metadata(target.parent().unwrap_or(Path::new("."))),
                ) {
                    if std::os::unix::fs::MetadataExt::dev(&a)
                        != std::os::unix::fs::MetadataExt::dev(&b)
                    {
                        Err("Move across filesystem volumes is unsupported".to_owned())
                    } else {
                        fs::rename(&current, target).map_err(|_| "Move failed".to_owned())
                    }
                } else {
                    Err("Source or destination is inaccessible".to_owned())
                }
            }
            Action::Copy { .. } => copy_atomic(&current, target),
        };
        match result {
            Ok(()) => {
                completed.push(label.clone());
                current = target.clone();
            }
            Err(error) => {
                final_error = Some(error);
                break;
            }
        }
    }
    let status = if final_error.is_some() {
        "failed"
    } else {
        "success"
    };
    record_outcome(
        rule,
        source,
        repo,
        completed,
        Some(current),
        status,
        final_error,
    )
}

fn record_outcome(
    rule: &Rule,
    source: &Path,
    repo: &RuleRepository,
    completed_actions: Vec<String>,
    resulting_path: Option<PathBuf>,
    status: &str,
    error: Option<String>,
) -> Result<(), String> {
    let intended_actions = rule.actions.iter().map(action_label).collect();
    let record = ActivityRecord {
        rule_id: rule.id.clone(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        source_path: source.to_string_lossy().to_string(),
        source_filename: source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        intended_actions,
        completed_actions,
        resulting_path: resulting_path.map(|path| path.to_string_lossy().to_string()),
        status: status.to_owned(),
        error: error.clone(),
    };
    let storage_result = repo.record_activity(record);
    if let Some(error) = error {
        return Err(error);
    }
    storage_result.map_err(|storage_error| {
        format!("Execution completed, but activity could not be saved: {storage_error}")
    })
}

fn action_label(action: &Action) -> String {
    match action {
        Action::Rename { pattern } => format!("Rename using {pattern}"),
        Action::Move { .. } => "Move".into(),
        Action::Copy { .. } => "Copy".into(),
    }
}

fn copy_atomic(source: &Path, destination: &Path) -> Result<(), String> {
    let temp = destination.with_file_name(format!(".teachmypc-{}-tmp", uuid::Uuid::new_v4()));
    fs::copy(source, &temp).map_err(|_| "Copy failed".to_owned())?;
    if destination.exists() {
        let _ = fs::remove_file(temp);
        return Err("Destination already exists".into());
    }
    fs::hard_link(&temp, destination).map_err(|_| "Could not safely finalize copy".to_owned())?;
    fs::remove_file(temp).map_err(|_| "Copied file but could not clean temporary file".to_owned())
}

#[derive(Default)]
struct WatcherLoopState {
    known: std::collections::HashMap<String, std::collections::HashSet<PathBuf>>,
    stable: std::collections::HashMap<PathBuf, (u64, u8)>,
    processed_file_ids: std::collections::HashMap<String, std::collections::HashSet<(u64, u64)>>,
    seeded: std::collections::HashSet<String>,
}

fn file_identity(metadata: &fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (metadata.dev(), metadata.ino())
}

fn poll_once(
    repo: &RuleRepository,
    state: &mut WatcherLoopState,
) -> std::collections::HashMap<String, RuleRuntimeStatus> {
    let mut next_statuses = std::collections::HashMap::new();
    if let Ok(rules) = repo.list() {
        let active: std::collections::HashSet<String> = rules
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.id.clone())
            .collect();
        state.known.retain(|id, _| active.contains(id));
        state.seeded.retain(|id| active.contains(id));
        state.processed_file_ids.retain(|id, _| active.contains(id));
        for rule in rules {
            if !rule.enabled {
                next_statuses.insert(
                    rule.id.clone(),
                    RuleRuntimeStatus {
                        rule_id: rule.id,
                        status: "Stopped".into(),
                        error: None,
                    },
                );
                continue;
            }
            let root = match repo.folder_path(&rule.source_folder_id) {
                Ok(root) => root,
                Err(error) => {
                    next_statuses.insert(
                        rule.id.clone(),
                        RuleRuntimeStatus {
                            rule_id: rule.id,
                            status: "Error".into(),
                            error: Some(error),
                        },
                    );
                    continue;
                }
            };
            let seen = state.known.entry(rule.id.clone()).or_default();
            let entries = match fs::read_dir(&root) {
                Ok(entries) => entries
                    .flatten()
                    .map(|entry| entry.path())
                    .collect::<Vec<_>>(),
                Err(error) => {
                    next_statuses.insert(
                        rule.id.clone(),
                        RuleRuntimeStatus {
                            rule_id: rule.id,
                            status: "Error".into(),
                            error: Some(format!("Could not read watched folder: {error}")),
                        },
                    );
                    continue;
                }
            };
            next_statuses.insert(
                rule.id.clone(),
                RuleRuntimeStatus {
                    rule_id: rule.id.clone(),
                    status: "Running".into(),
                    error: None,
                },
            );
            if state.seeded.insert(rule.id.clone()) {
                for path in entries {
                    if let Ok(metadata) = fs::symlink_metadata(&path) {
                        if metadata.is_file()
                            && !metadata.file_type().is_symlink()
                            && path_matches(&path, &rule)
                        {
                            seen.insert(path);
                            state
                                .processed_file_ids
                                .entry(rule.id.clone())
                                .or_default()
                                .insert(file_identity(&metadata));
                        }
                    }
                }
                continue;
            }
            let mut present_identities = std::collections::HashSet::new();
            for path in entries {
                let Ok(meta) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if !meta.is_file() || meta.file_type().is_symlink() {
                    continue;
                }
                let identity = file_identity(&meta);
                present_identities.insert(identity);
                if !path_matches(&path, &rule) {
                    continue;
                }
                let processed = state.processed_file_ids.entry(rule.id.clone()).or_default();
                if processed.contains(&identity) {
                    seen.insert(path.clone());
                    continue;
                }
                if seen.insert(path.clone()) {
                    state.stable.insert(path, (meta.len(), 0));
                    continue;
                }
                let key = path.clone();
                let length = meta.len();
                let Some((old_len, rounds)) = state.stable.get_mut(&key) else {
                    continue;
                };
                if *old_len == length {
                    *rounds = rounds.saturating_add(1);
                } else {
                    *old_len = length;
                    *rounds = 0;
                }
                if *rounds >= 2 {
                    let _ = execute(&rule, &path, &repo);
                    processed.insert(identity);
                    state.stable.remove(&key);
                }
            }
            if let Some(processed) = state.processed_file_ids.get_mut(&rule.id) {
                processed.retain(|identity| present_identities.contains(identity));
            }
        }
    }
    next_statuses
}

pub fn start(repo: Arc<RuleRepository>) -> WatcherManager {
    let statuses = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let thread_statuses = statuses.clone();
    thread::spawn(move || {
        let mut state = WatcherLoopState::default();
        loop {
            let next_statuses = poll_once(&repo, &mut state);
            if let Ok(mut current) = thread_statuses.lock() {
                *current = next_statuses;
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
    WatcherManager { statuses }
}
fn path_matches(path: &Path, rule: &Rule) -> bool {
    rule.trigger.extensions.is_empty()
        || path
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|ext| {
                rule.trigger
                    .extensions
                    .iter()
                    .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(ext))
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Trigger;
    fn fixture() -> (PathBuf, RuleRepository, Rule, PathBuf) {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("dest")).unwrap();
        let repo = RuleRepository::open(root.join("store.json")).unwrap();
        let source = repo.choose_folder(root.join("source")).unwrap();
        let dest = repo.choose_folder(root.join("dest")).unwrap();
        let sample = root.join("source/Invoice.PDF");
        fs::write(&sample, "fixture").unwrap();
        let rule = Rule {
            id: "test-rule".into(),
            title: "Test".into(),
            trigger: Trigger {
                kind: "file_created".into(),
                extensions: vec![".pdf".into()],
            },
            actions: vec![
                Action::Rename {
                    pattern: "{date}-{originalName}".into(),
                },
                Action::Move {
                    destination_folder_id: dest.id,
                },
            ],
            source_folder_id: source.id,
            enabled: true,
            created_at: 1,
            updated_at: 1,
        };
        (root, repo, rule, sample)
    }
    #[test]
    fn plan_preserves_extension_and_preview_does_not_mutate() {
        let (root, repo, rule, sample) = fixture();
        let preview = preview(&rule, &sample, &repo).unwrap();
        assert_eq!(preview.actions.len(), 2);
        assert!(preview.matches_rule);
        assert!(!preview.will_modify_files);
        assert!(sample.exists());
        assert!(preview.actions[0].ends_with("Invoice.PDF"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn preview_explains_non_matching_extension_without_mutating() {
        let (root, repo, mut rule, sample) = fixture();
        rule.trigger.extensions = vec![".png".into()];
        let preview = preview(&rule, &sample, &repo).unwrap();
        assert!(!preview.matches_rule);
        assert!(
            preview
                .message
                .as_deref()
                .unwrap()
                .contains("does not match")
        );
        assert!(preview.actions.is_empty());
        assert!(sample.exists());
        assert!(repo.activity().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn preview_rejects_outside_paths_and_reports_collisions_without_mutating() {
        let (root, repo, rule, sample) = fixture();
        let outside = root.join("outside.pdf");
        fs::write(&outside, "fixture").unwrap();
        assert!(
            preview(&rule, &outside, &repo)
                .unwrap_err()
                .contains("authorized")
        );
        let outside_mismatch = root.join("outside.png");
        fs::write(&outside_mismatch, "fixture").unwrap();
        let mut png_rule = rule.clone();
        png_rule.trigger.extensions = vec![".jpg".into()];
        assert!(
            preview(&png_rule, &outside_mismatch, &repo)
                .unwrap_err()
                .contains("authorized")
        );
        let renamed = format!("{}-Invoice.PDF", Local::now().format("%Y-%m-%d"));
        fs::write(root.join("source").join(renamed), "existing").unwrap();
        let collision = preview(&rule, &sample, &repo).unwrap();
        assert!(!collision.collisions.is_empty());
        assert!(sample.exists());
        assert!(repo.activity().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn preview_rejects_symbolic_link_samples() {
        let (root, repo, rule, sample) = fixture();
        let link = root.join("source/link.pdf");
        std::os::unix::fs::symlink(&sample, &link).unwrap();
        assert!(
            preview(&rule, &link, &repo)
                .unwrap_err()
                .contains("regular file")
        );
        assert!(sample.exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn ordered_rename_move_executes_and_records_activity() {
        let (root, repo, rule, sample) = fixture();
        execute(&rule, &sample, &repo).unwrap();
        assert!(!sample.exists());
        assert_eq!(repo.activity().unwrap()[0].status, "success");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copy_executes_and_persists_one_success_record() {
        let (root, repo, mut rule, sample) = fixture();
        let destination_folder = match &rule.actions[1] {
            Action::Move {
                destination_folder_id,
            } => destination_folder_id.clone(),
            _ => unreachable!(),
        };
        rule.actions = vec![Action::Copy {
            destination_folder_id: destination_folder,
        }];
        execute(&rule, &sample, &repo).unwrap();
        assert!(sample.is_file());
        assert!(root.join("dest/Invoice.PDF").is_file());
        assert_eq!(repo.activity().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn disabled_rules_do_not_execute() {
        let (root, repo, mut rule, sample) = fixture();
        rule.enabled = false;
        assert!(execute(&rule, &sample, &repo).is_err());
        assert!(sample.exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn extension_match_is_case_insensitive() {
        let (root, _, rule, sample) = fixture();
        assert!(path_matches(&sample, &rule));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn date_rename_preserves_original_extension_once_and_records_success() {
        let (root, repo, mut rule, sample) = fixture();
        let png = sample.with_file_name("test.png");
        fs::rename(&sample, &png).unwrap();
        rule.trigger.extensions = vec![".png".into()];
        rule.actions = vec![Action::Rename {
            pattern: "{date}-{originalName}".into(),
        }];
        execute(&rule, &png, &repo).unwrap();
        let expected = png.with_file_name(format!("{}-test.png", Local::now().format("%Y-%m-%d")));
        assert!(expected.is_file());
        assert_eq!(repo.activity().unwrap()[0].status, "success");
        let reopened = RuleRepository::open(root.join("store.json")).unwrap();
        assert_eq!(reopened.activity().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn original_name_placeholder_and_rename_only_work() {
        let (root, repo, mut rule, sample) = fixture();
        rule.actions = vec![Action::Rename {
            pattern: "{originalName}-copy".into(),
        }];
        execute(&rule, &sample, &repo).unwrap();
        assert!(root.join("source/Invoice-copy.PDF").is_file());
        assert!(!sample.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dry_run_detects_collisions_without_mutating_or_recording_activity() {
        let (root, repo, mut rule, sample) = fixture();
        rule.enabled = false;
        rule.actions = vec![Action::Rename {
            pattern: "{date}-{originalName}".into(),
        }];
        let target =
            sample.with_file_name(format!("{}-Invoice.PDF", Local::now().format("%Y-%m-%d")));
        fs::write(&target, "existing fixture").unwrap();
        let before = fs::read(&sample).unwrap();
        let preview = preview(&rule, &sample, &repo).unwrap();
        assert!(!preview.collisions.is_empty());
        assert_eq!(fs::read(&sample).unwrap(), before);
        assert_eq!(fs::read(&target).unwrap(), b"existing fixture");
        assert!(repo.activity().unwrap().is_empty());
        assert!(!rule.enabled);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copy_dry_run_reports_destination_without_copying() {
        let (root, repo, mut rule, sample) = fixture();
        rule.actions = vec![Action::Copy {
            destination_folder_id: rule
                .actions
                .iter()
                .find_map(|action| match action {
                    Action::Move {
                        destination_folder_id,
                    } => Some(destination_folder_id.clone()),
                    _ => None,
                })
                .unwrap(),
        }];
        let preview = preview(&rule, &sample, &repo).unwrap();
        assert_eq!(preview.actions.len(), 1);
        assert!(sample.exists());
        assert_eq!(fs::read_dir(root.join("dest")).unwrap().count(), 0);
        assert!(repo.activity().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn collision_is_persisted_as_skipped_and_planning_error_as_failed() {
        let (root, repo, mut rule, sample) = fixture();
        let target = root
            .join("dest")
            .join(format!("{}-Invoice.PDF", Local::now().format("%Y-%m-%d")));
        fs::write(&target, "fixture").unwrap();
        assert!(execute(&rule, &sample, &repo).is_err());
        assert_eq!(repo.activity().unwrap()[0].status, "skipped");

        rule.actions = vec![Action::Move {
            destination_folder_id: "missing-folder".into(),
        }];
        assert!(execute(&rule, &sample, &repo).is_err());
        let records = repo.activity().unwrap();
        assert_eq!(records[0].status, "failed");
        assert_eq!(records[1].status, "skipped");
        let reopened = RuleRepository::open(root.join("store.json")).unwrap();
        assert_eq!(reopened.activity().unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn watcher_ignores_initial_files_and_does_not_reprocess_renamed_output() {
        let (root, repo, mut rule, sample) = fixture();
        rule.trigger.extensions = vec![".png".into()];
        rule.actions = vec![Action::Rename {
            pattern: "{date}-{originalName}".into(),
        }];
        rule.id.clear();
        let saved = repo.save(rule).unwrap();
        repo.set_enabled(&saved.id, true).unwrap();
        let existing = root.join("source/existing.png");
        fs::write(&existing, "existing fixture").unwrap();
        let mut state = WatcherLoopState::default();
        assert_eq!(
            poll_once(&repo, &mut state).get(&saved.id).unwrap().status,
            "Running"
        );
        assert!(sample.exists());
        assert!(existing.exists());

        let new_file = root.join("source/test.png");
        fs::write(&new_file, "new fixture").unwrap();
        for _ in 0..3 {
            poll_once(&repo, &mut state);
        }
        let expected = root
            .join("source")
            .join(format!("{}-test.png", Local::now().format("%Y-%m-%d")));
        assert!(expected.is_file());
        for _ in 0..5 {
            poll_once(&repo, &mut state);
        }
        let matching: Vec<_> = fs::read_dir(root.join("source"))
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.ends_with(".png"))
            .collect();
        assert_eq!(matching.len(), 2);
        assert!(matching.contains(&"existing.png".to_owned()));
        assert!(matching.contains(&expected.file_name().unwrap().to_string_lossy().to_string()));
        assert_eq!(repo.activity().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enabling_survives_repository_restart_and_disable_delete_stops_status() {
        let (root, repo, mut rule, sample) = fixture();
        rule.id.clear();
        let saved = repo.save(rule).unwrap();
        repo.set_enabled(&saved.id, true).unwrap();
        let reopened = RuleRepository::open(root.join("store.json")).unwrap();
        assert!(reopened.list().unwrap()[0].enabled);
        let mut state = WatcherLoopState::default();
        assert_eq!(
            poll_once(&reopened, &mut state)
                .get(&saved.id)
                .unwrap()
                .status,
            "Running"
        );
        let mut edited = reopened.list().unwrap().remove(0);
        edited.title = "Edited title".into();
        reopened.save(edited).unwrap();
        assert_eq!(
            poll_once(&reopened, &mut state)
                .get(&saved.id)
                .unwrap()
                .status,
            "Stopped"
        );
        reopened.set_enabled(&saved.id, true).unwrap();
        assert_eq!(
            poll_once(&reopened, &mut state)
                .get(&saved.id)
                .unwrap()
                .status,
            "Running"
        );
        reopened.delete(&saved.id).unwrap();
        assert!(poll_once(&reopened, &mut state).is_empty());
        assert!(sample.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restored_watcher_ignores_existing_files_but_processes_new_files() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("dest")).unwrap();
        let repo_path = root.join("store.json");
        let repo = RuleRepository::open(repo_path.clone()).unwrap();
        let source = repo.choose_folder(root.join("source")).unwrap();
        let existing = root.join("source/existing.png");
        fs::write(&existing, "existing").unwrap();
        let rule = Rule {
            id: String::new(),
            title: "PNG dates".into(),
            trigger: Trigger {
                kind: "file_created".into(),
                extensions: vec![".png".into()],
            },
            actions: vec![Action::Rename {
                pattern: "{date}-{originalName}".into(),
            }],
            source_folder_id: source.id,
            enabled: false,
            created_at: 0,
            updated_at: 0,
        };
        let saved = repo.save(rule).unwrap();
        repo.set_enabled(&saved.id, true).unwrap();
        drop(repo);

        let restored = RuleRepository::open(repo_path).unwrap();
        assert!(restored.list().unwrap()[0].enabled);
        let mut state = WatcherLoopState::default();
        poll_once(&restored, &mut state);
        assert!(existing.exists());
        let new_file = root.join("source/test.png");
        fs::write(&new_file, "new").unwrap();
        for _ in 0..3 {
            poll_once(&restored, &mut state);
        }
        let expected = root
            .join("source")
            .join(format!("{}-test.png", Local::now().format("%Y-%m-%d")));
        assert!(expected.is_file());
        assert!(existing.is_file());
        assert_eq!(restored.activity().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
