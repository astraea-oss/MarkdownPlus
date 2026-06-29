# Workspace Spec

A MarkdownPlus workspace is a portable local directory.

```text
workspace/
  workspace.toml
  notes/
  bases/
  .local/
    index.sqlite
```

`notes/` stores native `.mdp` files named by stable note UUID.

`.local/index.sqlite` is a rebuildable index. The `.mdp` files remain the durable source of note content.

## Portable App-Owned Data

App-owned data that is not explicitly selected by the user must live under a single portable app data directory. By default this directory is created beside the executable:

```text
MarkdownPlusData/
  settings/
    settings.json
  runtime/
    config/
    data/
    cache/
```

The `MARKDOWNPLUS_PORTABLE_HOME` environment variable overrides this location.

The app must not intentionally store MarkdownPlus settings in platform AppData, XDG config/data/cache, or similar user-profile app directories. On Linux, the app sets `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and `XDG_CACHE_HOME` to subdirectories of the portable data folder before Tauri initializes.
