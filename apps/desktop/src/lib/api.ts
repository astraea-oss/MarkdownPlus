import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type {
  AppSettings,
  NoteDocument,
  NoteSource,
  NoteSummary,
  SaveNoteInput,
  SaveNoteSourceInput,
  WorkspaceSummary
} from './types';

export function getAppSettings(): Promise<AppSettings> {
  return invoke('get_app_settings');
}

export function openWorkspace(path: string): Promise<WorkspaceSummary> {
  return invoke('open_workspace', { path });
}

export function createNote(title?: string): Promise<NoteSummary> {
  return invoke('create_note', { input: { title: title || 'Untitled', note_type: 'note' } });
}

export function listNotes(): Promise<NoteSummary[]> {
  return invoke('list_notes');
}

export function getNote(id: string): Promise<NoteDocument> {
  return invoke('get_note', { id });
}

export function getNoteSource(id: string): Promise<NoteSource> {
  return invoke('get_note_source', { id });
}

export function saveNote(input: SaveNoteInput): Promise<{ note: NoteSummary }> {
  return invoke('save_note', { input });
}

export function saveNoteSource(input: SaveNoteSourceInput): Promise<{ note: NoteSummary }> {
  return invoke('save_note_source', { input });
}

export async function selectWorkspaceDirectory(defaultPath?: string): Promise<string | null> {
  const selected = await openDialog({
    title: 'Select MarkdownPlus workspace',
    directory: true,
    multiple: false,
    defaultPath: defaultPath || undefined
  });

  if (Array.isArray(selected)) {
    return selected[0] ?? null;
  }

  return selected;
}
