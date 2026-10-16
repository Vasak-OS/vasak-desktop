pub fn get_app_url() -> String {
  let is_dev: bool = tauri::is_dev();

  if is_dev {
      "http://localhost:1420/".to_string()
  } else {
      "tauri://localhost/".to_string()
  }
}
