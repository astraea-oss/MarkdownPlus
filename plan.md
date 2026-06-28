# Open-Source Local MarkdownPlus Notes App

## Summary

Build a fully local, AGPL-3.0 desktop notes/database app as an open-source alternative to Obsidian. The MVP is database-first: `.mdp` MarkdownPlus files with normalized YAML properties, stable note IDs, Bases-style table/card views, SQLite indexing, and a Tauri v2 + Rust + SvelteKit desktop shell.

## Core Decisions

- License: `AGPL-3.0`.
- App model: portable local workspace, no accounts, no telemetry, no network features by default.
- Stack: Rust core, Tauri v2 desktop app, SvelteKit frontend, TypeScript UI.
- Storage: human-readable `.mdp` files plus local SQLite index.
- Organization: no user-facing folders; all created notes go into one configured workspace note location.
- MVP focus: database/property/query experience first, with a functional editor.
- Search: SQLite FTS5.
- Identity: UUID note IDs; title is a mutable YAML property.
- Links: stable ID links with aliases, not title-based links.
- MarkdownPlus compatibility: mostly Markdown/CommonMark/GFM-compatible body text plus stricter YAML property schema.

## Workspace Layout

Use a portable workspace directory selected by the user:

```text
workspace/
  workspace.toml
  notes/
    <uuid>.mdp
  bases/
    <uuid>.base.yaml
  .local/
    index.sqlite
    thumbnails/
    logs/
```

`notes/` and `bases/` are implementation details, not navigation concepts in the UI.

## MarkdownPlus File Format

Native note extension: `.mdp`.

Each note has YAML frontmatter followed by Markdown-compatible body text:

```md
---
id: "018ff6e2-9f4b-7a64-b101-0e2fd6e32f20"
title: "Example note"
created_at: "2026-06-28T23:30:00Z"
updated_at: "2026-06-28T23:30:00Z"
tags: ["project"]
aliases: []
type: "note"
---

Body text here.
```

Required properties: `id`, `title`, `created_at`, `updated_at`.

Supported MVP property types: `string`, `number`, `boolean`, `date`, `datetime`, `string[]`, `number[]`, `link`, `link[]`, `json`.

Unknown YAML keys are preserved and indexed as untyped properties where possible.

## Bases-Style View Format

Store views as YAML files under `bases/`:

```yaml
id: "base-uuid"
name: "Projects"
source:
  type: "notes"
filters:
  - property: "type"
    op: "eq"
    value: "project"
sort:
  - property: "updated_at"
    direction: "desc"
views:
  - id: "table"
    type: "table"
    columns: ["title", "status", "updated_at"]
  - id: "cards"
    type: "cards"
    title: "title"
    subtitle: "status"
```

This is inspired by Obsidian Bases, whose `.base` files are YAML-backed view definitions, but the MVP should define its own stable schema rather than trying to clone every Obsidian feature immediately.

## Rust Crates / Modules

Create a Rust workspace:

```text
crates/
  mdp-core/      # file format, property model, parser types
  mdp-db/        # SQLite schema, migrations, query compiler
  mdp-indexer/   # scanner, FTS indexing, link extraction
  mdp-render/    # MarkdownPlus rendering/validation
apps/
  desktop/       # Tauri + SvelteKit app
docs/
  specs/
```

Recommended libraries:

- Markdown parsing/rendering: `comrak` for CommonMark/GFM compatibility.
- YAML: prefer `noyalib` because current Rust YAML options have changed and `serde_yaml` is no longer maintained.
- Database: `sqlx` with SQLite, migrations, and FTS5.
- IDs: UUID v7.
- File watching: `notify`.
- Tauri IPC: explicit Rust commands, not direct frontend SQL access.

## SQLite Schema

Initial tables:

```sql
notes(
  id text primary key,
  path text not null unique,
  title text not null,
  body_hash text not null,
  frontmatter_hash text not null,
  created_at text not null,
  updated_at text not null,
  indexed_at text not null
)

properties(
  note_id text not null,
  key text not null,
  type text not null,
  value_text text,
  value_number real,
  value_bool integer,
  value_datetime text,
  value_json text,
  primary key(note_id, key)
)

links(
  source_id text not null,
  target_id text not null,
  alias text,
  position_json text
)

notes_fts using fts5(title, body, content='notes')
bases(id text primary key, path text not null unique, name text not null)
```

## Tauri Command API

Expose typed commands only:

```ts
open_workspace(path: string): Promise<WorkspaceSummary>
create_note(input: CreateNoteInput): Promise<NoteSummary>
get_note(id: string): Promise<NoteDocument>
save_note(input: SaveNoteInput): Promise<SaveResult>
delete_note(id: string): Promise<void>
query_notes(query: QueryRequest): Promise<QueryResult>
list_properties(): Promise<PropertyDefinition[]>
create_base(input: BaseDefinition): Promise<BaseDefinition>
save_base(input: BaseDefinition): Promise<BaseDefinition>
render_markdownplus(input: RenderInput): Promise<RenderResult>
```

Frontend never writes notes or SQLite directly. Rust owns validation, atomic file writes, indexing, and migrations.

## UI MVP

Primary screens:

- Workspace picker/open recent.
- Notes database table.
- Card view for the same query.
- Note editor with YAML property panel and MarkdownPlus body editor.
- Base/view editor for filters, columns, sorting, and card fields.
- Search command palette.

Frontend implementation:

- SvelteKit + TypeScript.
- CodeMirror 6 for editor.
- TanStack Table or a lightweight equivalent for table view.
- No marketing/landing page; first screen is workspace selection or active workspace.

## Data Flow

1. User opens workspace.
2. Rust creates missing workspace directories and runs SQLite migrations.
3. Indexer scans `notes/*.mdp` and `bases/*.base.yaml`.
4. Parser validates YAML frontmatter and MarkdownPlus body.
5. SQLite stores note metadata, typed properties, extracted links, and FTS body text.
6. UI queries Rust command API.
7. On save, Rust writes via temp file + atomic rename, updates SQLite in one transaction, then emits workspace update events.

## Edge Cases

- Duplicate UUID: quarantine second file and surface repair action.
- Missing UUID: assign UUID on import, not silently on native `.mdp` corruption.
- Invalid YAML: keep file readable, mark note as invalid, exclude typed properties.
- Unknown property types: index as text/json fallback.
- Broken links: keep link record with unresolved target and show as unresolved.
- External file edits: file watcher triggers re-parse and updates UI.
- SQLite corruption: rebuild index from `.mdp` and `.base.yaml` files.
- Renames: no identity change, because filename is UUID and title is a property.

## Testing

Rust unit tests:

- MarkdownPlus frontmatter/body split.
- Property type normalization.
- UUID validation.
- Link extraction and alias parsing.
- Query compiler for filters/sorts.

Rust integration tests:

- Create/open workspace.
- Save note atomically.
- Rebuild SQLite index from files.
- Handle invalid YAML without data loss.
- FTS search returns expected notes.

Frontend tests:

- Create note flow.
- Edit properties and save.
- Table filtering/sorting.
- Card view rendering.
- Search result navigation.

End-to-end tests:

- Open workspace, create `.mdp`, edit YAML property, query by property, switch table/card view, close/reopen, verify persistence.

## Implementation Phases

1. Scaffold repo, AGPL license, Rust workspace, Tauri/SvelteKit app, CI.
2. Define MarkdownPlus and Base schemas in `docs/specs/`.
3. Implement workspace initialization, SQLite migrations, and file scanner.
4. Implement parser, property normalization, link extraction, and FTS indexing.
5. Add Tauri command API.
6. Build workspace picker, table view, card view, note editor, and property panel.
7. Add base/view editor.
8. Add import path for `.md` files into `.mdp`.
9. Add tests, packaging, and release artifacts.

## Acceptance Criteria

- A user can select a local workspace folder and create notes there.
- New notes are saved as `.mdp` files in the workspace note location.
- Notes have YAML properties and Markdown-compatible body text.
- Properties can be queried in table and card views.
- Full-text search works locally.
- Links survive title changes.
- The app can rebuild its SQLite index from files alone.
- No account, cloud, telemetry, or network dependency is required.

## Assumptions

- The working project name remains temporary until branding is chosen.
- `.mdp` is the native note format.
- Physical folders are internal workspace implementation, not a user organization model.
- Obsidian import is useful but not part of the first critical path beyond mostly compatible Markdown/YAML parsing.
- Mobile support is deferred; desktop comes first.

## References Checked

- Obsidian Bases overview: <https://help.obsidian.md/bases>
- Tauri v2 Rust command model: <https://v2.tauri.app/develop/calling-rust/>
- Tauri v2 filesystem model: <https://v2.tauri.app/plugin/file-system/>
- Current Rust YAML context: <https://docs.rs/serde-yaml> and <https://docs.rs/noyalib>
