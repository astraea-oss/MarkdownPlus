use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mdp_core::NoteDocument;
use mdp_workspace::{
    CreateNoteInput, NoteSource, NoteSummary, SaveNoteInput, SaveNoteSourceInput, SaveResult,
    WorkspaceHandle, WorkspaceSummary,
};
use serde::{Deserialize, Serialize};
use tauri::State;

struct AppState {
    workspace: Mutex<Option<WorkspaceHandle>>,
    settings: Mutex<AppSettings>,
    portable_root: PathBuf,
    settings_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    last_workspace_path: Option<String>,
    #[serde(default = "default_left_panel_width")]
    left_panel_width: u16,
    #[serde(default = "default_right_panel_width")]
    right_panel_width: u16,
    #[serde(default = "default_panel_mode")]
    left_panel_mode: String,
    #[serde(default = "default_panel_mode")]
    right_panel_mode: String,
    #[serde(default = "default_left_dock")]
    notes_dock: String,
    #[serde(default = "default_left_dock")]
    new_note_dock: String,
    #[serde(default = "default_left_dock")]
    settings_dock: String,
    #[serde(default = "default_right_dock")]
    outline_dock: String,
    #[serde(default = "default_top_tool_anchor")]
    notes_anchor: String,
    #[serde(default = "default_top_tool_anchor")]
    new_note_anchor: String,
    #[serde(default = "default_bottom_tool_anchor")]
    settings_anchor: String,
    #[serde(default = "default_top_tool_anchor")]
    outline_anchor: String,
    #[serde(default = "default_notes_order")]
    notes_order: u16,
    #[serde(default = "default_new_note_order")]
    new_note_order: u16,
    #[serde(default = "default_settings_order")]
    settings_order: u16,
    #[serde(default = "default_outline_order")]
    outline_order: u16,
    #[serde(default = "default_notes_hud_height")]
    notes_hud_height: u16,
    #[serde(default = "default_outline_hud_height")]
    outline_hud_height: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_workspace_path: None,
            left_panel_width: default_left_panel_width(),
            right_panel_width: default_right_panel_width(),
            left_panel_mode: default_panel_mode(),
            right_panel_mode: default_panel_mode(),
            notes_dock: default_left_dock(),
            new_note_dock: default_left_dock(),
            settings_dock: default_left_dock(),
            outline_dock: default_right_dock(),
            notes_anchor: default_top_tool_anchor(),
            new_note_anchor: default_top_tool_anchor(),
            settings_anchor: default_bottom_tool_anchor(),
            outline_anchor: default_top_tool_anchor(),
            notes_order: default_notes_order(),
            new_note_order: default_new_note_order(),
            settings_order: default_settings_order(),
            outline_order: default_outline_order(),
            notes_hud_height: default_notes_hud_height(),
            outline_hud_height: default_outline_hud_height(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct AppSettingsSummary {
    portable_root: String,
    last_workspace_path: Option<String>,
    left_panel_width: u16,
    right_panel_width: u16,
    left_panel_mode: String,
    right_panel_mode: String,
    notes_dock: String,
    new_note_dock: String,
    settings_dock: String,
    outline_dock: String,
    notes_anchor: String,
    new_note_anchor: String,
    settings_anchor: String,
    outline_anchor: String,
    notes_order: u16,
    new_note_order: u16,
    settings_order: u16,
    outline_order: u16,
    notes_hud_height: u16,
    outline_hud_height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateAppSettingsInput {
    left_panel_width: u16,
    right_panel_width: u16,
    left_panel_mode: String,
    right_panel_mode: String,
    notes_dock: String,
    new_note_dock: String,
    settings_dock: String,
    outline_dock: String,
    notes_anchor: String,
    new_note_anchor: String,
    settings_anchor: String,
    outline_anchor: String,
    notes_order: u16,
    new_note_order: u16,
    settings_order: u16,
    outline_order: u16,
    notes_hud_height: u16,
    outline_hud_height: u16,
}

impl AppState {
    fn load(portable_root: PathBuf) -> Self {
        let settings_dir = portable_root.join("settings");
        let settings_path = settings_dir.join("settings.json");
        let settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|source| serde_json::from_str::<AppSettings>(&source).ok())
            .unwrap_or_default();

        Self {
            workspace: Mutex::new(None),
            settings: Mutex::new(settings),
            portable_root,
            settings_path,
        }
    }

    fn save_settings(&self) -> Result<(), String> {
        let settings = self.settings.lock().map_err(lock_error)?.clone();
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let source = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
        fs::write(&self.settings_path, source).map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettingsSummary, String> {
    let settings = state.settings.lock().map_err(lock_error)?.clone();
    Ok(AppSettingsSummary {
        portable_root: state.portable_root.to_string_lossy().to_string(),
        last_workspace_path: settings.last_workspace_path,
        left_panel_width: settings.left_panel_width,
        right_panel_width: settings.right_panel_width,
        left_panel_mode: settings.left_panel_mode,
        right_panel_mode: settings.right_panel_mode,
        notes_dock: settings.notes_dock,
        new_note_dock: settings.new_note_dock,
        settings_dock: settings.settings_dock,
        outline_dock: settings.outline_dock,
        notes_anchor: settings.notes_anchor,
        new_note_anchor: settings.new_note_anchor,
        settings_anchor: settings.settings_anchor,
        outline_anchor: settings.outline_anchor,
        notes_order: settings.notes_order,
        new_note_order: settings.new_note_order,
        settings_order: settings.settings_order,
        outline_order: settings.outline_order,
        notes_hud_height: settings.notes_hud_height,
        outline_hud_height: settings.outline_hud_height,
    })
}

#[tauri::command]
fn update_app_settings(
    input: UpdateAppSettingsInput,
    state: State<'_, AppState>,
) -> Result<AppSettingsSummary, String> {
    {
        let mut settings = state.settings.lock().map_err(lock_error)?;
        settings.left_panel_width = input.left_panel_width.clamp(220, 420);
        settings.right_panel_width = input.right_panel_width.clamp(210, 420);
        settings.left_panel_mode = normalize_panel_mode(&input.left_panel_mode).to_string();
        settings.right_panel_mode = normalize_panel_mode(&input.right_panel_mode).to_string();
        settings.notes_dock = normalize_dock_side(&input.notes_dock).to_string();
        settings.new_note_dock = normalize_dock_side(&input.new_note_dock).to_string();
        settings.settings_dock = normalize_dock_side(&input.settings_dock).to_string();
        settings.outline_dock = normalize_dock_side(&input.outline_dock).to_string();
        settings.notes_anchor = normalize_tool_anchor(&input.notes_anchor).to_string();
        settings.new_note_anchor = normalize_tool_anchor(&input.new_note_anchor).to_string();
        settings.settings_anchor = normalize_tool_anchor(&input.settings_anchor).to_string();
        settings.outline_anchor = normalize_tool_anchor(&input.outline_anchor).to_string();
        settings.notes_order = input.notes_order.min(1000);
        settings.new_note_order = input.new_note_order.min(1000);
        settings.settings_order = input.settings_order.min(1000);
        settings.outline_order = input.outline_order.min(1000);
        settings.notes_hud_height = input.notes_hud_height.clamp(96, 720);
        settings.outline_hud_height = input.outline_hud_height.clamp(96, 720);
    }

    state.save_settings()?;
    get_app_settings(state)
}

#[tauri::command]
fn open_workspace(path: String, state: State<'_, AppState>) -> Result<WorkspaceSummary, String> {
    let workspace = WorkspaceHandle::open(path).map_err(to_command_error)?;
    let summary = workspace.summary().map_err(to_command_error)?;
    *state.workspace.lock().map_err(lock_error)? = Some(workspace);
    state
        .settings
        .lock()
        .map_err(lock_error)?
        .last_workspace_path = Some(summary.root.clone());
    state.save_settings()?;
    Ok(summary)
}

#[tauri::command]
fn create_note(input: CreateNoteInput, state: State<'_, AppState>) -> Result<NoteSummary, String> {
    let guard = state.workspace.lock().map_err(lock_error)?;
    let workspace = guard.as_ref().ok_or("open a workspace first")?;
    workspace.create_note(input).map_err(to_command_error)
}

#[tauri::command]
fn list_notes(state: State<'_, AppState>) -> Result<Vec<NoteSummary>, String> {
    let guard = state.workspace.lock().map_err(lock_error)?;
    let workspace = guard.as_ref().ok_or("open a workspace first")?;
    workspace.list_notes().map_err(to_command_error)
}

#[tauri::command]
fn get_note(id: String, state: State<'_, AppState>) -> Result<NoteDocument, String> {
    let guard = state.workspace.lock().map_err(lock_error)?;
    let workspace = guard.as_ref().ok_or("open a workspace first")?;
    workspace.get_note(&id).map_err(to_command_error)
}

#[tauri::command]
fn get_note_source(id: String, state: State<'_, AppState>) -> Result<NoteSource, String> {
    let guard = state.workspace.lock().map_err(lock_error)?;
    let workspace = guard.as_ref().ok_or("open a workspace first")?;
    workspace.get_note_source(&id).map_err(to_command_error)
}

#[tauri::command]
fn save_note(input: SaveNoteInput, state: State<'_, AppState>) -> Result<SaveResult, String> {
    let guard = state.workspace.lock().map_err(lock_error)?;
    let workspace = guard.as_ref().ok_or("open a workspace first")?;
    workspace.save_note(input).map_err(to_command_error)
}

#[tauri::command]
fn save_note_source(
    input: SaveNoteSourceInput,
    state: State<'_, AppState>,
) -> Result<SaveResult, String> {
    let guard = state.workspace.lock().map_err(lock_error)?;
    let workspace = guard.as_ref().ok_or("open a workspace first")?;
    workspace.save_note_source(input).map_err(to_command_error)
}

pub fn run() {
    let portable_root = configure_portable_environment();

    tauri::Builder::default()
        .manage(AppState::load(portable_root))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            update_app_settings,
            open_workspace,
            create_note,
            list_notes,
            get_note,
            get_note_source,
            save_note,
            save_note_source
        ])
        .run(tauri::generate_context!())
        .expect("error while running MarkdownPlus");
}

fn to_command_error(error: anyhow::Error) -> String {
    error.to_string()
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("application state lock failed: {error}")
}

fn default_left_panel_width() -> u16 {
    285
}

fn default_right_panel_width() -> u16 {
    255
}

fn default_panel_mode() -> String {
    "view".to_string()
}

fn default_top_tool_anchor() -> String {
    "top".to_string()
}

fn default_bottom_tool_anchor() -> String {
    "bottom".to_string()
}

fn default_notes_order() -> u16 {
    10
}

fn default_new_note_order() -> u16 {
    20
}

fn default_settings_order() -> u16 {
    30
}

fn default_outline_order() -> u16 {
    40
}

fn default_left_dock() -> String {
    "left".to_string()
}

fn default_right_dock() -> String {
    "right".to_string()
}

fn default_notes_hud_height() -> u16 {
    320
}

fn default_outline_hud_height() -> u16 {
    190
}

fn normalize_panel_mode(mode: &str) -> &str {
    match mode {
        "ribbon" => "ribbon",
        _ => "view",
    }
}

fn normalize_tool_anchor(anchor: &str) -> &str {
    match anchor {
        "center" => "center",
        "bottom" => "bottom",
        _ => "top",
    }
}

fn normalize_dock_side(side: &str) -> &str {
    match side {
        "right" => "right",
        _ => "left",
    }
}

fn configure_portable_environment() -> PathBuf {
    let portable_root = std::env::var_os("MARKDOWNPLUS_PORTABLE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(default_portable_root);

    let runtime_root = portable_root.join("runtime");
    let _ = fs::create_dir_all(portable_root.join("settings"));
    let _ = fs::create_dir_all(runtime_root.join("config"));
    let _ = fs::create_dir_all(runtime_root.join("data"));
    let _ = fs::create_dir_all(runtime_root.join("cache"));
    let _ = fs::create_dir_all(runtime_root.join("temp"));

    configure_platform_runtime_environment(&portable_root);

    portable_root
}

#[cfg(target_os = "linux")]
fn configure_platform_runtime_environment(portable_root: &Path) {
    let runtime_root = portable_root.join("runtime");

    // Keep framework/webview runtime data near the executable instead of the
    // user's XDG app-data/cache locations. This must happen before Tauri
    // initializes windows or plugins.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", runtime_root.join("config"));
        std::env::set_var("XDG_DATA_HOME", runtime_root.join("data"));
        std::env::set_var("XDG_CACHE_HOME", runtime_root.join("cache"));
        std::env::set_var("TMPDIR", runtime_root.join("temp"));
    }
}

#[cfg(target_os = "windows")]
fn configure_platform_runtime_environment(portable_root: &Path) {
    let runtime_root = portable_root.join("runtime");

    // Route framework/webview runtime data into the portable folder on Windows.
    // WebView2 and common Windows crates use these variables as their app-data
    // roots, so set them before Tauri initializes windows or plugins.
    unsafe {
        std::env::set_var("APPDATA", runtime_root.join("config"));
        std::env::set_var("LOCALAPPDATA", runtime_root.join("data"));
        std::env::set_var("TEMP", runtime_root.join("temp"));
        std::env::set_var("TMP", runtime_root.join("temp"));
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn configure_platform_runtime_environment(portable_root: &Path) {
    let runtime_root = portable_root.join("runtime");

    unsafe {
        std::env::set_var("TMPDIR", runtime_root.join("temp"));
    }
}

fn default_portable_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("MarkdownPlusData")
}
