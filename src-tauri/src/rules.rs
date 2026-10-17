use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub title: String,
    pub trigger: Trigger,
    pub actions: Vec<Action>,
    pub source_folder_id: String,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Trigger {
    #[serde(rename = "type")]
    pub kind: String,
    pub extensions: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    Rename { pattern: String },
    Move { destination_folder_id: String },
    Copy { destination_folder_id: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoreFile {
    version: u32,
    folders: HashMap<String, PathBuf>,
    rules: Vec<Rule>,
    #[serde(default)]
    activity: Vec<ActivityRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRecord {
    pub rule_id: String,
    pub timestamp: u64,
    pub source_path: String,
    pub source_filename: String,
    pub intended_actions: Vec<String>,
    pub completed_actions: Vec<String>,
    pub resulting_path: Option<String>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct RuleRepository {
    path: PathBuf,
    data: Mutex<StoreFile>,
    unavailable: Option<String>,
}
impl RuleRepository {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let data = if path.exists() {
            let bytes = fs::read(&path).map_err(|_| "Could not read rule storage".to_owned())?;
            let store: StoreFile = serde_json::from_slice(&bytes)
                .map_err(|_| "Rule storage is malformed; its contents were preserved".to_owned())?;
            if store.version != 1 {
                return Err(
                    "Rule storage version is unsupported; its contents were preserved".into(),
                );
            }
            validate_store(&store)?;
            store
        } else {
            StoreFile {
                version: 1,
                folders: HashMap::new(),
                rules: Vec::new(),
                activity: Vec::new(),
            }
        };
        Ok(Self {
            path,
            data: Mutex::new(data),
            unavailable: None,
        })
    }
    pub fn unavailable(path: PathBuf, reason: String) -> Self {
        Self {
            path,
            data: Mutex::new(StoreFile {
                version: 1,
                folders: HashMap::new(),
                rules: Vec::new(),
                activity: Vec::new(),
            }),
            unavailable: Some(reason),
        }
    }
    pub fn choose_folder(&self, path: PathBuf) -> Result<FolderChoice, String> {
        self.check_available()?;
        let canonical = path
            .canonicalize()
            .map_err(|_| "Selected folder is unavailable".to_owned())?;
        if !canonical.is_dir() || fs::read_dir(&canonical).is_err() {
            return Err("Selected folder is not accessible".into());
        }
        let id = new_id();
        let name = canonical
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Folder")
            .to_owned();
        self.data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?
            .folders
            .insert(id.clone(), canonical);
        Ok(FolderChoice { id, name })
    }
    pub fn list(&self) -> Result<Vec<Rule>, String> {
        self.check_available()?;
        Ok(self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?
            .rules
            .clone())
    }
    pub fn folders(&self) -> Result<Vec<FolderChoice>, String> {
        self.check_available()?;
        let data = self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?;
        Ok(data
            .folders
            .iter()
            .map(|(id, path)| FolderChoice {
                id: id.clone(),
                name: path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Folder")
                    .to_owned(),
            })
            .collect())
    }
    pub fn save(&self, mut rule: Rule) -> Result<Rule, String> {
        self.check_available()?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?;
        validate_rule(&rule, &data.folders)?;
        let now = now();
        if rule.id.is_empty() {
            rule.id = new_id();
            rule.created_at = now;
        } else if let Some(old) = data.rules.iter().find(|r| r.id == rule.id) {
            rule.created_at = old.created_at;
        } else {
            return Err("Rule ID was not found".into());
        }
        rule.updated_at = now;
        rule.enabled = false;
        if let Some(index) = data.rules.iter().position(|r| r.id == rule.id) {
            data.rules[index] = rule.clone();
        } else {
            data.rules.push(rule.clone());
        }
        self.write(&data)?;
        Ok(rule)
    }
    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.check_available()?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?;
        let old_len = data.rules.len();
        data.rules.retain(|r| r.id != id);
        if data.rules.len() == old_len {
            return Err("Rule was not found".into());
        }
        self.write(&data)
    }
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<Rule, String> {
        self.check_available()?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?;
        let folders = data.folders.clone();
        let rule = data
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Rule was not found")?;
        if enabled {
            validate_rule(rule, &folders)?;
        }
        rule.enabled = enabled;
        rule.updated_at = now();
        let result = rule.clone();
        self.write(&data)?;
        Ok(result)
    }
    pub fn folder_path(&self, id: &str) -> Result<PathBuf, String> {
        self.check_available()?;
        let data = self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?;
        let path = data
            .folders
            .get(id)
            .ok_or("Authorized folder was not found")?;
        let canonical = path
            .canonicalize()
            .map_err(|_| "Authorized folder is unavailable")?;
        if !canonical.is_dir() || fs::read_dir(&canonical).is_err() {
            return Err("Authorized folder is not accessible".into());
        }
        Ok(canonical)
    }
    pub fn action_folder(&self, id: &str) -> Result<PathBuf, String> {
        self.folder_path(id)
    }
    pub fn record_activity(&self, record: ActivityRecord) -> Result<(), String> {
        self.check_available()?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?;
        data.activity.push(record);
        if data.activity.len() > 500 {
            let excess = data.activity.len() - 500;
            data.activity.drain(0..excess);
        }
        self.write(&data)
    }
    pub fn activity(&self) -> Result<Vec<ActivityRecord>, String> {
        self.check_available()?;
        let data = self
            .data
            .lock()
            .map_err(|_| "Rule storage is unavailable".to_owned())?;
        Ok(data.activity.iter().rev().take(100).cloned().collect())
    }
    fn write(&self, data: &StoreFile) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| "Could not create rule storage".to_owned())?;
        }
        let temp = self.path.with_extension(format!("tmp-{}", new_id()));
        let bytes =
            serde_json::to_vec_pretty(data).map_err(|_| "Could not encode rules".to_owned())?;
        fs::write(&temp, bytes).map_err(|_| "Could not write temporary rule storage".to_owned())?;
        fs::rename(&temp, &self.path).map_err(|_| {
            let _ = fs::remove_file(&temp);
            "Could not replace rule storage".to_owned()
        })
    }
    fn check_available(&self) -> Result<(), String> {
        self.unavailable.clone().map_or(Ok(()), Err)
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderChoice {
    pub id: String,
    pub name: String,
}
fn validate_store(store: &StoreFile) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for (id, path) in &store.folders {
        if id.is_empty() || !path.is_absolute() || !path.is_dir() {
            return Err("Stored folder reference is invalid; storage was preserved".into());
        }
    }
    for rule in &store.rules {
        if !ids.insert(&rule.id) {
            return Err("Duplicate rule ID in storage; storage was preserved".into());
        }
        validate_rule(rule, &store.folders)
            .map_err(|_| "Stored rule is invalid; storage was preserved".to_owned())?;
    }
    Ok(())
}
fn validate_rule(rule: &Rule, folders: &HashMap<String, PathBuf>) -> Result<(), String> {
    if rule.title.trim().is_empty() || rule.title.chars().count() > 80 {
        return Err("Enter a rule title under 80 characters".into());
    }
    if rule.trigger.kind != "file_created" {
        return Err("Only file-created triggers are supported".into());
    }
    if rule.trigger.extensions.len() > 32
        || rule.trigger.extensions.iter().any(|e| {
            e.len() < 2
                || e.len() > 16
                || !e.starts_with('.')
                || !e[1..].chars().all(|c| c.is_ascii_alphanumeric())
        })
    {
        return Err("One or more file extensions are invalid".into());
    }
    if rule.actions.is_empty() || rule.actions.len() > 8 {
        return Err("Add between one and eight actions".into());
    }
    let source = folders
        .get(&rule.source_folder_id)
        .ok_or("Select an authorized source folder")?
        .canonicalize()
        .map_err(|_| "Source folder is unavailable")?;
    if !source.is_dir() || fs::read_dir(&source).is_err() {
        return Err("Source folder is not accessible".into());
    }
    for action in &rule.actions {
        match action {
            Action::Rename { pattern } => {
                if pattern.is_empty()
                    || pattern.len() > 128
                    || !valid_pattern(pattern)
                    || pattern_has_extension(pattern)
                {
                    return Err("Rename template contains an unsupported placeholder".into());
                }
            }
            Action::Move {
                destination_folder_id,
            }
            | Action::Copy {
                destination_folder_id,
            } => {
                let dest = folders
                    .get(destination_folder_id)
                    .ok_or("Select a destination folder for each move or copy")?
                    .canonicalize()
                    .map_err(|_| "Destination folder is unavailable")?;
                if !dest.is_dir()
                    || fs::read_dir(&dest).is_err()
                    || dest.starts_with(&source)
                    || source.starts_with(&dest)
                {
                    return Err("Source and destination folders cannot contain one another".into());
                }
            }
        }
    }
    Ok(())
}
fn valid_pattern(value: &str) -> bool {
    let mut rest = value;
    while let Some(start) = rest.find('{') {
        rest = &rest[start..];
        let Some(end) = rest.find('}') else {
            return false;
        };
        let token = &rest[..=end];
        if token != "{originalName}" && token != "{date}" {
            return false;
        }
        rest = &rest[end + 1..];
    }
    !rest.contains('}')
}
fn pattern_has_extension(value: &str) -> bool {
    value.rsplit_once('.').is_some_and(|(_, suffix)| {
        !suffix.is_empty()
            && suffix
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
    })
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        std::env::temp_dir().join(new_id())
    }
    #[test]
    fn save_edit_delete_roundtrip_and_order() {
        let root = temp();
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("dest")).unwrap();
        let repo = RuleRepository::open(root.join("rules.json")).unwrap();
        let src = repo.choose_folder(root.join("source")).unwrap();
        let dst = repo.choose_folder(root.join("dest")).unwrap();
        let saved = repo
            .save(Rule {
                id: String::new(),
                title: "Invoices".into(),
                trigger: Trigger {
                    kind: "file_created".into(),
                    extensions: vec![".pdf".into()],
                },
                actions: vec![
                    Action::Rename {
                        pattern: "{date}-{originalName}".into(),
                    },
                    Action::Move {
                        destination_folder_id: dst.id,
                    },
                ],
                source_folder_id: src.id,
                enabled: false,
                created_at: 0,
                updated_at: 0,
            })
            .unwrap();
        assert!(matches!(saved.actions[0], Action::Rename { .. }));
        let edited = repo
            .save(Rule {
                title: "Updated".into(),
                ..saved.clone()
            })
            .unwrap();
        assert_eq!(edited.title, "Updated");
        let reloaded = RuleRepository::open(root.join("rules.json")).unwrap();
        assert_eq!(reloaded.list().unwrap().len(), 1);
        reloaded.delete(&edited.id).unwrap();
        assert!(reloaded.list().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn malformed_storage_is_preserved() {
        let root = temp();
        fs::create_dir_all(&root).unwrap();
        let path = root.join("rules.json");
        fs::write(&path, b"bad").unwrap();
        assert!(RuleRepository::open(path.clone()).is_err());
        assert_eq!(fs::read(path).unwrap(), b"bad");
        fs::remove_dir_all(root).unwrap();
    }

    fn setup() -> (PathBuf, RuleRepository, FolderChoice, FolderChoice) {
        let root = temp();
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("destination")).unwrap();
        let repo = RuleRepository::open(root.join("rules.json")).unwrap();
        let source = repo.choose_folder(root.join("source")).unwrap();
        let destination = repo.choose_folder(root.join("destination")).unwrap();
        (root, repo, source, destination)
    }
    fn draft(source: &FolderChoice, actions: Vec<Action>) -> Rule {
        Rule {
            id: String::new(),
            title: "PDF filing".into(),
            trigger: Trigger {
                kind: "file_created".into(),
                extensions: vec![".pdf".into()],
            },
            actions,
            source_folder_id: source.id.clone(),
            enabled: true,
            created_at: 0,
            updated_at: 0,
        }
    }
    #[test]
    fn rejects_missing_source_and_unresolved_destination() {
        let (root, repo, source, _) = setup();
        let mut no_source = draft(
            &source,
            vec![Action::Rename {
                pattern: "{originalName}".into(),
            }],
        );
        no_source.source_folder_id = "unapproved".into();
        assert!(repo.save(no_source).unwrap_err().contains("source folder"));
        assert!(
            repo.save(draft(
                &source,
                vec![Action::Move {
                    destination_folder_id: "unresolved".into()
                }]
            ))
            .unwrap_err()
            .contains("destination")
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_invalid_rename_extensions_and_unsafe_folder_pairs() {
        let (root, repo, source, destination) = setup();
        assert!(
            repo.save(draft(
                &source,
                vec![Action::Rename {
                    pattern: "{unknown}".into()
                }]
            ))
            .unwrap_err()
            .contains("placeholder")
        );
        assert!(
            repo.save(draft(
                &source,
                vec![Action::Rename {
                    pattern: "{originalName}.pdf".into()
                }]
            ))
            .unwrap_err()
            .contains("placeholder")
        );
        let nested_path = root.join("source/nested");
        fs::create_dir(&nested_path).unwrap();
        let nested = repo.choose_folder(nested_path).unwrap();
        assert!(
            repo.save(draft(
                &source,
                vec![Action::Copy {
                    destination_folder_id: nested.id
                }]
            ))
            .unwrap_err()
            .contains("cannot contain")
        );
        assert!(
            repo.save(draft(&source, vec![]))
                .unwrap_err()
                .contains("one and eight")
        );
        assert!(
            repo.save(draft(
                &source,
                vec![Action::Move {
                    destination_folder_id: destination.id
                }]
            ))
            .is_ok()
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_malformed_ipc_payloads_and_unknown_action_fields() {
        let bad = r#"{"id":"x","title":"x","trigger":{"type":"file_created","extensions":[]},"actions":[{"type":"launch","app":"x"}],"sourceFolderId":"x","enabled":false,"createdAt":0,"updatedAt":0,"extra":true}"#;
        assert!(serde_json::from_str::<Rule>(bad).is_err());
    }
    #[test]
    fn save_always_disables_rules_and_preserves_order() {
        let (root, repo, source, destination) = setup();
        let saved = repo
            .save(draft(
                &source,
                vec![
                    Action::Rename {
                        pattern: "{date}-{originalName}".into(),
                    },
                    Action::Move {
                        destination_folder_id: destination.id,
                    },
                ],
            ))
            .unwrap();
        assert!(!saved.enabled);
        assert!(matches!(saved.actions[0], Action::Rename { .. }));
        assert!(matches!(saved.actions[1], Action::Move { .. }));
        assert!(
            repo.save(Rule {
                id: "forged-id".into(),
                ..draft(
                    &source,
                    vec![Action::Rename {
                        pattern: "{originalName}".into()
                    }]
                )
            })
            .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
