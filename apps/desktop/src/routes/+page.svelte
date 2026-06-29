<script lang="ts">
  import { onMount } from 'svelte';
  import DOMPurify from 'dompurify';
  import { marked } from 'marked';
  import {
    createNote,
    getAppSettings,
    getNoteSource,
    listNotes,
    openWorkspace,
    saveNoteSource,
    selectWorkspaceDirectory
  } from '$lib/api';
  import type { NoteSource, NoteSummary, WorkspaceSummary } from '$lib/types';

  type EditorMode = 'write' | 'split' | 'preview';

  let workspacePath = '';
  let workspace: WorkspaceSummary | null = null;
  let notes: NoteSummary[] = [];
  let selectedNoteSource: NoteSource | null = null;
  let noteSource = '';
  let status = 'Choose or type a workspace path to begin.';
  let portableRoot = '';
  let saving = false;
  let browsing = false;
  let settingsOpen = false;
  let editorMode: EditorMode = 'split';

  $: selectedId = selectedNoteSource?.id;
  $: selectedTitle = notes.find((note) => note.id === selectedId)?.title ?? 'Untitled';
  $: markdownHtml = DOMPurify.sanitize(
    marked.parse(extractMarkdownBody(noteSource), { async: false }) as string
  );

  onMount(async () => {
    const settings = await getAppSettings();
    portableRoot = settings.portable_root;

    if (settings.last_workspace_path) {
      workspacePath = settings.last_workspace_path;
      try {
        await openWorkspacePath(settings.last_workspace_path);
      } catch (error) {
        settingsOpen = true;
        status = error instanceof Error ? error.message : String(error);
      }
    } else {
      settingsOpen = true;
      status = `Portable data: ${settings.portable_root}`;
    }
  });

  async function openCurrentWorkspace() {
    if (!workspacePath.trim()) {
      status = 'Enter a local workspace path.';
      return;
    }

    await openWorkspacePath(workspacePath.trim());
  }

  async function browseWorkspace() {
    browsing = true;
    const selected = await selectWorkspaceDirectory(workspacePath.trim());
    browsing = false;

    if (!selected) {
      status = 'Workspace selection cancelled.';
      return;
    }

    workspacePath = selected;
    await openWorkspacePath(selected);
  }

  async function openWorkspacePath(path: string) {
    workspace = await openWorkspace(path);
    notes = await listNotes();
    selectedNoteSource = null;
    noteSource = '';
    settingsOpen = false;
    status = `Opened ${workspace.root}`;
  }

  async function createNewNote() {
    const note = await createNote('Untitled');
    notes = await listNotes();
    await selectNote(note.id);
    status = 'Created note.';
  }

  async function selectNote(id: string) {
    selectedNoteSource = await getNoteSource(id);
    noteSource = selectedNoteSource.source;
    status = 'Note loaded.';
  }

  async function saveSelectedNote() {
    if (!selectedNoteSource) return;

    saving = true;
    try {
      const result = await saveNoteSource({
        id: selectedNoteSource.id,
        source: noteSource
      });
      notes = await listNotes();
      await selectNote(result.note.id);
      status = 'Saved.';
    } catch (error) {
      status = error instanceof Error ? error.message : String(error);
    } finally {
      saving = false;
    }
  }

  function extractMarkdownBody(source: string): string {
    if (!source.startsWith('---')) {
      return source;
    }

    const delimiter = source.indexOf('\n---', 3);
    if (delimiter === -1) {
      return '';
    }

    return source.slice(delimiter + 4).replace(/^\r?\n+/, '');
  }
</script>

<svelte:head>
  <title>MarkdownPlus</title>
</svelte:head>

<main class="app-shell">
  <aside class="sidebar">
    <div class="brand" data-tauri-drag-region>
      <div>
        <h1>MarkdownPlus</h1>
        <p>Local .mdp workspace</p>
      </div>
    </div>

    {#if workspace}
      <section class="notes-header">
        <div>
          <span class="count">{notes.length}</span>
          <span>notes</span>
        </div>
        <button on:click={createNewNote}>New</button>
      </section>

      <nav class="notes-list" aria-label="Notes">
        {#each notes as note}
          <button
            class:active={note.id === selectedId}
            class="note-row"
            on:click={() => selectNote(note.id)}
          >
            <span class="note-title">{note.title}</span>
            <span class="note-meta">{note.note_type} · {new Date(note.updated_at).toLocaleString()}</span>
          </button>
        {/each}
      </nav>
    {:else}
      <div class="sidebar-empty">
        Open Settings to choose a workspace.
      </div>
    {/if}

    <div class="sidebar-footer">
      {#if settingsOpen}
        <section class="settings-panel">
          <div class="settings-heading">
            <h2>Settings</h2>
            <button class="icon-button" aria-label="Close settings" on:click={() => (settingsOpen = false)}>
              X
            </button>
          </div>

          <label for="workspace-path">Workspace folder</label>
          <div class="workspace-row">
            <input
              id="workspace-path"
              bind:value={workspacePath}
              placeholder="/home/lua/MarkdownPlus Workspace"
            />
            <button on:click={browseWorkspace} disabled={browsing}>
              {browsing ? '...' : 'Browse'}
            </button>
            <button class="primary" on:click={openCurrentWorkspace}>Open</button>
          </div>
          {#if portableRoot}
            <p class="settings-note">Portable app data: {portableRoot}</p>
          {/if}
        </section>
      {/if}

      <button class:active={settingsOpen} class="settings-button" on:click={() => (settingsOpen = !settingsOpen)}>
        Settings
      </button>
    </div>
  </aside>

  <section class="editor">
    {#if selectedNoteSource}
      <header class="editor-header">
        <div>
          <h2>{selectedTitle}</h2>
          <p>{selectedNoteSource.id}</p>
        </div>
        <button class="primary" disabled={saving} on:click={saveSelectedNote}>
          {saving ? 'Saving' : 'Save'}
        </button>
      </header>

      <div class="editor-toolbar">
        <div class="mode-group" aria-label="Editor mode">
          <button
            class:active={editorMode === 'write'}
            class="mode-button"
            on:click={() => (editorMode = 'write')}
          >
            Write
          </button>
          <button
            class:active={editorMode === 'split'}
            class="mode-button"
            on:click={() => (editorMode = 'split')}
          >
            Split
          </button>
          <button
            class:active={editorMode === 'preview'}
            class="mode-button"
            on:click={() => (editorMode = 'preview')}
          >
            Preview
          </button>
        </div>
      </div>

      <div class:preview-only={editorMode === 'preview'} class:write-only={editorMode === 'write'} class="body-shell">
        {#if editorMode !== 'preview'}
          <textarea class="body-editor" bind:value={noteSource} aria-label="MarkdownPlus source"></textarea>
        {/if}

        {#if editorMode !== 'write'}
          <article class="markdown-preview">
            {@html markdownHtml}
          </article>
        {/if}
      </div>
    {:else}
      <div class="empty-state" data-tauri-drag-region>
        <h2>Open a workspace</h2>
        <p>Use Settings in the sidebar to select a local workspace folder.</p>
      </div>
    {/if}

    <footer>{status}</footer>
  </section>
</main>

<style>
  .app-shell {
    display: grid;
    grid-template-columns: minmax(240px, 285px) minmax(0, 1fr);
    min-height: 100vh;
    max-height: 100vh;
    background: #0d1117;
  }

  .sidebar {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: 0.72rem;
    border-right: 1px solid #232b36;
    background: #0b0f14;
    padding: 0.72rem;
    min-height: 0;
  }

  .brand h1 {
    margin: 0;
    color: #f0f4f8;
    font-size: 1rem;
    line-height: 1.15;
  }

  .brand p,
  .editor-header p,
  .empty-state p,
  footer {
    margin: 0.15rem 0 0;
    color: #7d8896;
    font-size: 0.75rem;
  }

  .workspace-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.38rem;
  }

  .notes-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    color: #aeb8c4;
    font-size: 0.78rem;
  }

  .count {
    font-weight: 700;
    color: #d7dde4;
  }

  .notes-list {
    display: grid;
    align-content: start;
    gap: 0.22rem;
    overflow: auto;
    min-height: 0;
  }

  .sidebar-empty {
    display: grid;
    align-content: start;
    color: #7d8896;
    font-size: 0.78rem;
    line-height: 1.35;
  }

  .sidebar-footer {
    display: grid;
    gap: 0.5rem;
  }

  .settings-panel {
    display: grid;
    gap: 0.45rem;
    border: 1px solid #232b36;
    border-radius: 6px;
    background: #0f141b;
    padding: 0.58rem;
  }

  .settings-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .settings-heading h2 {
    margin: 0;
    color: #e6edf3;
    font-size: 0.82rem;
  }

  .settings-panel label {
    color: #9aa6b2;
    font-size: 0.72rem;
    font-weight: 650;
  }

  .settings-note {
    overflow: hidden;
    margin: 0;
    color: #7d8896;
    font-size: 0.68rem;
    line-height: 1.3;
    text-overflow: ellipsis;
  }

  .settings-button {
    width: 100%;
    text-align: left;
  }

  .settings-button.active {
    border-color: #2ea987;
    background: #10211e;
  }

  .icon-button {
    display: grid;
    place-items: center;
    width: 1.55rem;
    height: 1.55rem;
    padding: 0;
    font-size: 0.72rem;
  }

  .note-row {
    display: grid;
    gap: 0.16rem;
    width: 100%;
    text-align: left;
    background: #0f141b;
    border-color: transparent;
    padding: 0.46rem 0.52rem;
  }

  .note-row.active {
    background: #10211e;
    border-color: #2ea987;
  }

  .note-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #e6edf3;
    font-weight: 650;
  }

  .note-meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #7d8896;
    font-size: 0.72rem;
  }

  .editor {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    gap: 0.55rem;
    padding: 0.72rem;
    min-width: 0;
    min-height: 0;
    background: #0d1117;
  }

  .editor-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    gap: 0.72rem;
  }

  .editor-header h2 {
    overflow: hidden;
    margin: 0;
    color: #f0f4f8;
    font-size: 1.16rem;
    font-weight: 700;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .mode-group {
    display: inline-grid;
    grid-template-columns: repeat(3, auto);
    gap: 0.22rem;
    border: 1px solid #232b36;
    border-radius: 6px;
    background: #0b0f14;
    padding: 0.18rem;
  }

  .mode-button {
    border-color: transparent;
    padding: 0.24rem 0.48rem;
    font-size: 0.76rem;
  }

  .mode-button.active {
    border-color: #2ea987;
    background: #10211e;
    color: #e6edf3;
  }

  .body-shell {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 0.55rem;
    min-height: 0;
  }

  .body-shell.write-only,
  .body-shell.preview-only {
    grid-template-columns: minmax(0, 1fr);
  }

  .body-editor {
    min-height: 0;
    height: 100%;
    line-height: 1.35;
    border-color: #232b36;
    background: #0b0f14;
    tab-size: 2;
    font-family:
      "SFMono-Regular", Consolas, "Liberation Mono", Menlo, ui-monospace, monospace;
    font-size: 0.88rem;
  }

  .markdown-preview {
    min-height: 0;
    overflow: auto;
    border: 1px solid #232b36;
    border-radius: 5px;
    background: #0b0f14;
    padding: 0.65rem 0.72rem;
    color: #d7dde4;
    line-height: 1.35;
    font-size: 0.9rem;
  }

  .markdown-preview :global(*) {
    max-width: 100%;
  }

  .markdown-preview :global(h1),
  .markdown-preview :global(h2),
  .markdown-preview :global(h3) {
    margin: 0 0 0.28rem;
    color: #f0f4f8;
    line-height: 1.15;
  }

  .markdown-preview :global(h1) {
    font-size: 1.08rem;
  }

  .markdown-preview :global(h2) {
    font-size: 0.98rem;
  }

  .markdown-preview :global(h3) {
    font-size: 0.92rem;
  }

  .markdown-preview :global(p),
  .markdown-preview :global(ul),
  .markdown-preview :global(ol),
  .markdown-preview :global(blockquote),
  .markdown-preview :global(pre) {
    margin: 0 0 0.28rem;
  }

  .markdown-preview :global(strong) {
    color: #f0f4f8;
    font-weight: 750;
  }

  .markdown-preview :global(em) {
    color: #b9c7d5;
  }

  .markdown-preview :global(code) {
    border: 1px solid #303946;
    border-radius: 4px;
    background: #111820;
    padding: 0.08rem 0.28rem;
    color: #8bd5bd;
    font-family:
      "SFMono-Regular", Consolas, "Liberation Mono", Menlo, ui-monospace, monospace;
    font-size: 0.88em;
  }

  .markdown-preview :global(pre) {
    overflow: auto;
    border: 1px solid #303946;
    border-radius: 5px;
    background: #111820;
    padding: 0.45rem;
  }

  .markdown-preview :global(pre code) {
    border: 0;
    background: transparent;
    padding: 0;
  }

  .markdown-preview :global(a) {
    color: #4fbda0;
  }

  .empty-state {
    display: grid;
    place-content: center;
    min-height: 100%;
    text-align: center;
    color: #d7dde4;
  }

  .empty-state h2 {
    margin: 0 0 0.35rem;
    font-size: 1.1rem;
  }

  footer {
    overflow: hidden;
    border-top: 1px solid #232b36;
    padding-top: 0.5rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 760px) {
    .app-shell {
      grid-template-columns: 1fr;
    }

    .sidebar {
      border-right: 0;
      border-bottom: 1px solid #232b36;
    }

  }
</style>
