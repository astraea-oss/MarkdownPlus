# MarkdownPlus

MarkdownPlus is a fully local, open-source notes and knowledge-base app inspired by Obsidian, built around portable `.mdp` files and database-style views.

The current implementation is the first vertical slice:

- Tauri v2 desktop shell.
- SvelteKit frontend.
- Rust workspace core.
- Portable local workspace layout.
- `.mdp` files with YAML frontmatter and Markdown body.
- SQLite index for note summaries and properties.
- Basic create, list, open, edit, and save note flow.
- Portable app settings stored beside the executable, not in AppData/XDG app folders.

## Development Prerequisites

- Rust stable 1.85 or newer.
- Node.js and npm.
- Linux Tauri system dependencies for your distribution.

## Run

```bash
npm --prefix apps/desktop install
npm --prefix apps/desktop run tauri dev
```

## Workspace Layout

When the app opens a workspace path, it creates:

```text
workspace/
  workspace.toml
  notes/
  bases/
  .local/
    index.sqlite
```

Created notes are stored as `notes/<uuid>.mdp`.

## Portable App Data

MarkdownPlus keeps app-owned settings and runtime directories in a portable folder beside the executable:

```text
MarkdownPlusData/
  settings/
    settings.json
  runtime/
    config/
    data/
    cache/
```

Set `MARKDOWNPLUS_PORTABLE_HOME` before launching the app to override that folder.

The selected workspace is user-owned and can live anywhere. Once selected, its path is cached in `MarkdownPlusData/settings/settings.json` and reopened automatically on the next launch.
