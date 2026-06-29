use std::fs;
use std::path::PathBuf;
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AppSettings {
    last_workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AppSettingsSummary {
    portable_root: String,
    last_workspace_path: Option<String>,
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
    })
}

#[tauri::command]
fn open_workspace(path: String, state: State<'_, AppState>) -> Result<WorkspaceSummary, String> {
    let workspace = WorkspaceHandle::open(path).map_err(to_command_error)?;
    let summary = workspace.summary().map_err(to_command_error)?;
    *state.workspace.lock().map_err(lock_error)? = Some(workspace);
    state.settings.lock().map_err(lock_error)?.last_workspace_path = Some(summary.root.clone());
    state.save_settings()?;
    Ok(summary)
}

#[tauri::command]
fn create_note(
    input: CreateNoteInput,
    state: State<'_, AppState>,
) -> Result<NoteSummary, String> {
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

fn configure_portable_environment() -> PathBuf {
    let portable_root = std::env::var_os("MARKDOWNPLUS_PORTABLE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(default_portable_root);

    let _ = fs::create_dir_all(portable_root.join("settings"));
    let _ = fs::create_dir_all(portable_root.join("runtime").join("config"));
    let _ = fs::create_dir_all(portable_root.join("runtime").join("data"));
    let _ = fs::create_dir_all(portable_root.join("runtime").join("cache"));

    // Keep framework/webview runtime data near the executable on Linux instead of
    // the user's XDG app-data/cache locations. This must happen before Tauri
    // initializes windows or plugins.
    unsafe {
        std::env::set_var(
            "XDG_CONFIG_HOME",
            portable_root.join("runtime").join("config"),
        );
        std::env::set_var("XDG_DATA_HOME", portable_root.join("runtime").join("data"));
        std::env::set_var("XDG_CACHE_HOME", portable_root.join("runtime").join("cache"));
    }

    portable_root
}

fn default_portable_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("MarkdownPlusData")
}
