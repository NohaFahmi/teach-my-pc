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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![get_app_info])
    .run(tauri::generate_context!())
    .expect("error while building TeachMyPC");
}
