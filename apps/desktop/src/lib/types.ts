export type WorkspaceSummary = {
  root: string;
  note_count: number;
};

export type AppSettings = {
  portable_root: string;
  last_workspace_path: string | null;
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
