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

mod interpreter;

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
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            interpret_instruction
        ])
        .run(tauri::generate_context!())
        .expect("error while building TeachMyPC");
}
