#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: &'static str,
    version: &'static str,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "TeachMyPC",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalAiStatus {
    endpoint: &'static str,
    model: &'static str,
    connected: bool,
    model_available: bool,
}

fn local_ai_model_available(tags: &serde_json::Value, model: &str) -> bool {
    tags.get("models")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|models| {
            models.iter().any(|entry| {
                entry
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|name| name == model || name.starts_with(&format!("{model}:")))
            })
        })
}

#[tauri::command]
fn get_local_ai_status() -> LocalAiStatus {
    let tags = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(700))
        .timeout(std::time::Duration::from_millis(1200))
        .build()
        .ok()
        .and_then(|client| client.get("http://127.0.0.1:11434/api/tags").send().ok())
        .filter(|response| response.status().is_success())
        .and_then(|response| response.json::<serde_json::Value>().ok());
    let connected = tags.is_some();
    let model_available = tags
        .as_ref()
        .is_some_and(|tags| local_ai_model_available(tags, "gemma3:4b"));
    LocalAiStatus {
        endpoint: "http://127.0.0.1:11434",
        model: "gemma3:4b",
        connected,
        model_available,
    }
}

#[cfg(test)]
mod local_ai_status_tests {
    use super::local_ai_model_available;

    #[test]
    fn detects_configured_model_in_ollama_tags() {
        let tags = serde_json::json!({"models": [{"name": "gemma3:4b"}, {"name": "llama3:8b"}]});
        assert!(local_ai_model_available(&tags, "gemma3:4b"));
        assert!(!local_ai_model_available(&tags, "mistral:7b"));
        assert!(!local_ai_model_available(
            &serde_json::json!({"models": []}),
            "gemma3:4b"
        ));
    }
}

mod engine;
mod interpreter;
mod rules;
use tauri::Manager;

#[tauri::command]
fn list_rules(
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<Vec<rules::Rule>, String> {
    repository.list()
}

#[tauri::command]
fn list_authorized_folders(
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<Vec<rules::FolderChoice>, String> {
    repository.folders()
}

#[tauri::command]
fn save_rule(
    rule: rules::Rule,
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<rules::Rule, String> {
    repository.save(rule)
}

#[tauri::command]
fn delete_rule(
    id: String,
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<(), String> {
    repository.delete(&id)
}

#[tauri::command]
fn set_rule_enabled(
    id: String,
    enabled: bool,
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<rules::Rule, String> {
    repository.set_enabled(&id, enabled)
}
#[tauri::command]
fn list_activity(
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<Vec<rules::ActivityRecord>, String> {
    repository.activity()
}
#[tauri::command]
fn list_rule_runtime_statuses(
    watcher: tauri::State<'_, engine::WatcherManager>,
) -> Result<Vec<engine::RuleRuntimeStatus>, String> {
    watcher.list()
}
#[tauri::command]
fn preview_rule(
    id: String,
    sample_path: String,
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<engine::PlanPreview, String> {
    let rule = repository
        .list()?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or("Rule was not found")?;
    let sample = std::path::PathBuf::from(sample_path);
    engine::preview(&rule, &sample, &repository)
}

#[tauri::command]
async fn choose_preview_file(
    id: String,
    app: tauri::AppHandle,
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let rule = repository
        .list()?
        .into_iter()
        .find(|rule| rule.id == id)
        .ok_or("Rule was not found")?;
    let source = repository.folder_path(&rule.source_folder_id)?;
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    app.dialog()
        .file()
        .set_title("Choose a sample file for Dry Run")
        .set_directory(&source)
        .pick_file(move |picked| {
            let _ = sender.try_send(picked);
        });
    let picked = receiver
        .recv()
        .await
        .ok_or_else(|| "File selection was interrupted".to_owned())?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|_| "Selected file path is invalid".to_owned())?;
    if std::fs::symlink_metadata(&path)
        .map_err(|_| "Selected file is unavailable".to_owned())?
        .file_type()
        .is_symlink()
    {
        return Err("Symbolic links cannot be used for Dry Run".into());
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| "Selected file is unavailable".to_owned())?;
    if !canonical.starts_with(source) || !canonical.is_file() {
        return Err("Choose a regular file inside this rule's authorized source folder".into());
    }
    Ok(Some(canonical.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn choose_folder(
    app: tauri::AppHandle,
    repository: tauri::State<'_, std::sync::Arc<rules::RuleRepository>>,
) -> Result<Option<rules::FolderChoice>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    app.dialog()
        .file()
        .set_title("Choose an authorized folder")
        .pick_folder(move |picked| {
            let _ = sender.try_send(picked);
        });
    let picked = receiver
        .recv()
        .await
        .ok_or_else(|| "Folder selection was interrupted".to_owned())?;
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|_| "Selected folder path is invalid".to_owned())?;
    repository.choose_folder(path).map(Some)
}

#[tauri::command]
async fn interpret_instruction(
    instruction: String,
) -> Result<interpreter::InterpretationResult, interpreter::InterpretationError> {
    tauri::async_runtime::spawn_blocking(move || interpreter::interpret_instruction(&instruction))
        .await
        .map_err(|_| interpreter::InterpretationError::Internal)?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store_path = data_dir.join("rules-v1.json");
            let repository =
                rules::RuleRepository::open(store_path.clone()).unwrap_or_else(|error| {
                    rules::RuleRepository::unavailable(store_path.clone(), error)
                });
            let repository = std::sync::Arc::new(repository);
            let watcher = engine::start(repository.clone());
            app.manage(repository);
            app.manage(watcher);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_local_ai_status,
            interpret_instruction,
            list_rules,
            list_authorized_folders,
            save_rule,
            delete_rule,
            set_rule_enabled,
            list_activity,
            list_rule_runtime_statuses,
            preview_rule,
            choose_preview_file,
            choose_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while building TeachMyPC");
}
