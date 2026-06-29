export type WorkspaceSummary = {
  root: string;
  note_count: number;
};

export type AppSettings = {
  portable_root: string;
  last_workspace_path: string | null;
  left_panel_width: number;
  right_panel_width: number;
  left_panel_mode: PanelMode;
  right_panel_mode: PanelMode;
  notes_dock: DockSide;
  new_note_dock: DockSide;
  settings_dock: DockSide;
  outline_dock: DockSide;
  notes_anchor: ToolAnchor;
  new_note_anchor: ToolAnchor;
  settings_anchor: ToolAnchor;
  outline_anchor: ToolAnchor;
  notes_order: number;
  new_note_order: number;
  settings_order: number;
  outline_order: number;
  notes_hud_height: number;
  outline_hud_height: number;
};

export type PanelMode = 'view' | 'ribbon';
export type ToolAnchor = 'top' | 'center' | 'bottom';
export type DockSide = 'left' | 'right';

export type UpdateAppSettingsInput = {
  left_panel_width: number;
  right_panel_width: number;
  left_panel_mode: PanelMode;
  right_panel_mode: PanelMode;
  notes_dock: DockSide;
  new_note_dock: DockSide;
  settings_dock: DockSide;
  outline_dock: DockSide;
  notes_anchor: ToolAnchor;
  new_note_anchor: ToolAnchor;
  settings_anchor: ToolAnchor;
  outline_anchor: ToolAnchor;
  notes_order: number;
  new_note_order: number;
  settings_order: number;
  outline_order: number;
  notes_hud_height: number;
  outline_hud_height: number;
};

export type NoteSummary = {
  id: string;
  title: string;
  note_type: string;
  updated_at: string;
  path: string;
};

export type NoteDocument = {
  frontmatter: {
    id: string;
    title: string;
    created_at: string;
    updated_at: string;
    tags: string[];
    aliases: string[];
    type: string;
    [key: string]: unknown;
  };
  body: string;
};

export type NoteSource = {
  id: string;
  source: string;
};

export type SaveNoteInput = {
  id: string;
  title: string;
  note_type: string;
  tags: string[];
  aliases: string[];
  body: string;
};

export type SaveNoteSourceInput = {
  id: string;
  source: string;
};
