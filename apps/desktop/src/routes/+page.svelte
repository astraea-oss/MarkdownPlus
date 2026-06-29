<script lang="ts">
  import { onMount } from 'svelte';
  import DOMPurify from 'dompurify';
  import { marked } from 'marked';
  import MarkdownPlusEditor from '$lib/MarkdownPlusEditor.svelte';
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

  type EditorMode = 'live' | 'source' | 'split' | 'preview';
  type PropertyRow = {
    key: string;
    value: string;
  };
  type OutlineItem = {
    level: number;
    text: string;
  };

  const minLeftPanelWidth = 220;
  const maxLeftPanelWidth = 420;

  let leftPanelWidth = 285;
  let workspacePath = '';
  let workspace: WorkspaceSummary | null = null;
  let notes: NoteSummary[] = [];
  let selectedNoteSource: NoteSource | null = null;
  let noteSource = '';
  let liveBody = '';
  let propertyRows: PropertyRow[] = [];
  let status = 'Choose or type a workspace path to begin.';
  let portableRoot = '';
  let saving = false;
  let browsing = false;
  let settingsOpen = false;
  let editorMode: EditorMode = 'live';

  $: selectedId = selectedNoteSource?.id;
  $: selectedTitle = notes.find((note) => note.id === selectedId)?.title ?? 'Untitled';
  $: outlineItems = extractOutline(extractMarkdownBody(noteSource));
  $: markdownHtml = DOMPurify.sanitize(
    marked.parse(markdownPlusPreviewSource(extractMarkdownBody(noteSource)), {
      async: false,
      breaks: true
    }) as string
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
    syncLiveFieldsFromSource();
    status = 'Note loaded.';
  }

  async function saveSelectedNote() {
    if (!selectedNoteSource) return;

    saving = true;
    try {
      if (editorMode === 'live') {
        updateSourceFromLiveFields();
      }

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

  function markdownPlusPreviewSource(source: string): string {
    return source.replace(/^[ \t]*-{3,}[ \t]*$/gm, '\n<hr data-mdp-rule="underline">\n');
  }

  function extractOutline(source: string): OutlineItem[] {
    return source
      .split(/\r?\n/)
      .map((line) => line.match(/^\s*(#{1,3})\s+(.+)$/))
      .filter((match): match is RegExpMatchArray => Boolean(match))
      .map((match) => ({
        level: match[1].length,
        text: match[2].replace(/[*_`[\]]/g, '').trim()
      }))
      .filter((item) => item.text);
  }

  function startLeftPanelResize(event: PointerEvent) {
    event.preventDefault();

    const startX = event.clientX;
    const startWidth = leftPanelWidth;
    document.body.classList.add('is-resizing-panel');

    const resizePanel = (moveEvent: PointerEvent) => {
      leftPanelWidth = clamp(startWidth + moveEvent.clientX - startX, minLeftPanelWidth, maxLeftPanelWidth);
    };

    const stopResize = () => {
      document.body.classList.remove('is-resizing-panel');
      window.removeEventListener('pointermove', resizePanel);
      window.removeEventListener('pointerup', stopResize);
      window.removeEventListener('pointercancel', stopResize);
    };

    window.addEventListener('pointermove', resizePanel);
    window.addEventListener('pointerup', stopResize);
    window.addEventListener('pointercancel', stopResize);
  }

  function handleLeftPanelResizeKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      leftPanelWidth = clamp(leftPanelWidth - 12, minLeftPanelWidth, maxLeftPanelWidth);
    }

    if (event.key === 'ArrowRight') {
      event.preventDefault();
      leftPanelWidth = clamp(leftPanelWidth + 12, minLeftPanelWidth, maxLeftPanelWidth);
    }
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function syncLiveFieldsFromSource() {
    const split = splitMarkdownPlusSource(noteSource);
    liveBody = split.body;
    propertyRows = split.frontmatter
      .split(/\r?\n/)
      .map((line) => line.match(/^([A-Za-z0-9_-]+):\s*(.*)$/))
      .filter((match): match is RegExpMatchArray => Boolean(match))
      .map((match) => ({
        key: match[1],
        value: match[2] ?? ''
      }));
  }

  function updateSourceFromLiveFields() {
    const yaml = propertyRows
      .filter((property) => property.key.trim())
      .map((property) => `${property.key.trim()}: ${property.value}`)
      .join('\n');

    noteSource = `---\n${yaml}\n---\n${liveBody}`;
  }

  function updateProperty(index: number, field: keyof PropertyRow, value: string) {
    propertyRows = propertyRows.map((property, propertyIndex) =>
      propertyIndex === index ? { ...property, [field]: value } : property
    );
    updateSourceFromLiveFields();
  }

  function addProperty() {
    propertyRows = [...propertyRows, { key: 'property', value: 'value' }];
    updateSourceFromLiveFields();
  }

  function removeProperty(index: number) {
    propertyRows = propertyRows.filter((_, propertyIndex) => propertyIndex !== index);
    updateSourceFromLiveFields();
  }

  function updateLiveBody(value: string) {
    liveBody = value;
    updateSourceFromLiveFields();
  }

  function splitMarkdownPlusSource(source: string): { frontmatter: string; body: string } {
    if (!source.startsWith('---')) {
      return { frontmatter: '', body: source };
    }

    const delimiter = source.indexOf('\n---', 3);
    if (delimiter === -1) {
      return { frontmatter: '', body: source };
    }

    const frontmatterStart = source.startsWith('---\r\n') ? 5 : 4;
    return {
      frontmatter: source.slice(frontmatterStart, delimiter),
      body: source.slice(delimiter + 4).replace(/^\r?\n+/, '')
    };
  }

  function handleEditorKeydown(event: KeyboardEvent, target: 'source' | 'body' = 'source') {
    if (event.key !== 'Tab') {
      return;
    }

    event.preventDefault();
    const textarea = event.currentTarget as HTMLTextAreaElement;
    const selectionStart = textarea.selectionStart;
    const selectionEnd = textarea.selectionEnd;

    if (event.shiftKey) {
      outdentSelection(textarea, selectionStart, selectionEnd, target);
      return;
    }

    indentSelection(textarea, selectionStart, selectionEnd, target);
  }

  function handleSourceInput(value: string) {
    noteSource = value;
    syncLiveFieldsFromSource();
  }

  function setEditorText(target: 'source' | 'body', value: string) {
    if (target === 'body') {
      updateLiveBody(value);
      return;
    }

    handleSourceInput(value);
  }

  function indentSelection(
    textarea: HTMLTextAreaElement,
    selectionStart: number,
    selectionEnd: number,
    target: 'source' | 'body'
  ) {
    const indent = '  ';
    const text = target === 'body' ? liveBody : noteSource;

    if (selectionStart === selectionEnd) {
      setEditorText(target, text.slice(0, selectionStart) + indent + text.slice(selectionEnd));
      queueSelection(textarea, selectionStart + indent.length, selectionStart + indent.length);
      return;
    }

    const lineStart = text.lastIndexOf('\n', selectionStart - 1) + 1;
    const selected = text.slice(lineStart, selectionEnd);
    const indented = selected.replace(/^/gm, indent);
    setEditorText(target, text.slice(0, lineStart) + indented + text.slice(selectionEnd));
    queueSelection(textarea, selectionStart + indent.length, selectionEnd + indented.length - selected.length);
  }

  function outdentSelection(
    textarea: HTMLTextAreaElement,
    selectionStart: number,
    selectionEnd: number,
    target: 'source' | 'body'
  ) {
    const text = target === 'body' ? liveBody : noteSource;
    const lineStart = text.lastIndexOf('\n', selectionStart - 1) + 1;
    const selected = text.slice(lineStart, selectionEnd);
    const outdented = selected.replace(/^( {1,2}|\t)/gm, '');
    const removedBeforeSelection = selected
      .slice(0, selectionStart - lineStart)
      .match(/^( {1,2}|\t)/gm)
      ?.join('').length ?? 0;

    setEditorText(target, text.slice(0, lineStart) + outdented + text.slice(selectionEnd));

    const removedTotal = selected.length - outdented.length;
    queueSelection(
      textarea,
      Math.max(lineStart, selectionStart - removedBeforeSelection),
      Math.max(lineStart, selectionEnd - removedTotal)
    );
  }

  function queueSelection(textarea: HTMLTextAreaElement, start: number, end: number) {
    requestAnimationFrame(() => {
      textarea.selectionStart = start;
      textarea.selectionEnd = end;
    });
  }
</script>

<svelte:head>
  <title>MarkdownPlus</title>
</svelte:head>

<main class="app-shell" style={`--left-panel-width: ${leftPanelWidth}px;`}>
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

  <button
    class="panel-resize-handle"
    type="button"
    aria-label="Resize left panel"
    on:pointerdown={startLeftPanelResize}
    on:keydown={handleLeftPanelResizeKeydown}
  ></button>

  <section class="editor">
    {#if selectedNoteSource}
      <div class="note-page">
        <header class="editor-header">
          <div>
            <h2>{selectedTitle}</h2>
            <p>{selectedNoteSource.id}</p>
          </div>
          <button class="primary" disabled={saving} on:click={saveSelectedNote}>
            {saving ? 'Saving' : 'Save'}
          </button>
        </header>

        {#if editorMode === 'live'}
          <div class="live-editor">
            <section class="live-properties" aria-label="Note properties">
              <div class="property-list">
                {#each propertyRows as property, index}
                  <div class="property-row">
                    <input
                      aria-label="Property name"
                      value={property.key}
                      on:input={(event) => updateProperty(index, 'key', event.currentTarget.value)}
                    />
                    <input
                      aria-label="Property value"
                      value={property.value}
                      on:input={(event) => updateProperty(index, 'value', event.currentTarget.value)}
                    />
                    <button class="property-remove-button" aria-label="Remove property" on:click={() => removeProperty(index)}>
                      X
                    </button>
                  </div>
                {/each}
              </div>

              <button class="add-property-button" on:click={addProperty}>Add property</button>
            </section>

            <div class="live-body-editor">
              <MarkdownPlusEditor
                value={liveBody}
                ariaLabel="MarkdownPlus body"
                onChange={updateLiveBody}
              />
            </div>
          </div>
        {:else}
          <div
            class:preview-only={editorMode === 'preview'}
            class:source-only={editorMode === 'source'}
            class="body-shell"
          >
            {#if editorMode !== 'preview'}
            <textarea
              class="body-editor"
              value={noteSource}
              aria-label="MarkdownPlus source"
              on:input={(event) => handleSourceInput(event.currentTarget.value)}
              on:keydown={handleEditorKeydown}
            ></textarea>
            {/if}

            {#if editorMode !== 'source'}
            <article class="markdown-preview">
              {@html markdownHtml}
            </article>
            {/if}
          </div>
        {/if}
      </div>
    {:else}
      <div class="empty-state" data-tauri-drag-region>
        <h2>Open a workspace</h2>
        <p>Use Settings in the sidebar to select a local workspace folder.</p>
      </div>
    {/if}

    <footer>
      <span>{status}</span>
      {#if selectedNoteSource}
        <div class="mode-group" aria-label="Editor mode">
          <button
            class:active={editorMode === 'live'}
            class="mode-button"
            on:click={() => (editorMode = 'live')}
          >
            Live
          </button>
          <button
            class:active={editorMode === 'source'}
            class="mode-button"
            on:click={() => (editorMode = 'source')}
          >
            Source
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
      {/if}
    </footer>
  </section>

  <aside class="right-panel">
    <div class="right-panel-toolbar" data-tauri-drag-region>
      <h2>Outline</h2>
    </div>

    {#if selectedNoteSource && outlineItems.length}
      <nav class="outline-list" aria-label="Note outline">
        {#each outlineItems as item}
          <div class:level-two={item.level === 2} class:level-three={item.level === 3} class="outline-row">
            {item.text}
          </div>
        {/each}
      </nav>
    {:else}
      <div class="right-panel-empty">
        No headings found.
      </div>
    {/if}
  </aside>
</main>

<style>
  .app-shell {
    display: grid;
    grid-template-columns: var(--left-panel-width) 4px minmax(0, 1fr) minmax(210px, 255px);
    min-height: 100vh;
    max-height: 100vh;
    background: #0d1117;
  }

  .sidebar {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: 0.72rem;
    background: #0b0f14;
    padding: 0.72rem;
    min-height: 0;
  }

  .panel-resize-handle {
    width: 4px;
    min-width: 4px;
    height: 100vh;
    border: 0;
    border-right: 1px solid #232b36;
    border-left: 1px solid transparent;
    border-radius: 0;
    background: #0b0f14;
    cursor: col-resize;
    padding: 0;
  }

  .panel-resize-handle:hover,
  .panel-resize-handle:focus {
    border-left-color: #2ea987;
    border-right-color: #2ea987;
    outline: none;
  }

  :global(body.is-resizing-panel),
  :global(body.is-resizing-panel *) {
    cursor: col-resize !important;
    user-select: none !important;
  }

  .right-panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.54rem;
    border-left: 1px solid #232b36;
    background: #0b0f14;
    padding: 0.72rem 0.64rem;
    min-width: 0;
    min-height: 0;
  }

  .right-panel-toolbar {
    display: flex;
    align-items: center;
    min-height: 1.7rem;
  }

  .right-panel-toolbar h2 {
    margin: 0;
    color: #d7dde4;
    font-size: 0.78rem;
    font-weight: 650;
  }

  .outline-list {
    display: grid;
    align-content: start;
    gap: 0.08rem;
    min-height: 0;
    overflow: auto;
  }

  .outline-row {
    overflow: hidden;
    border-radius: 4px;
    padding: 0.22rem 0.18rem;
    color: #aeb8c4;
    font-size: 0.76rem;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .outline-row.level-two {
    padding-left: 0.76rem;
    color: #9aa6b2;
  }

  .outline-row.level-three {
    padding-left: 1.28rem;
    color: #8d98a6;
  }

  .outline-row:hover {
    background: #10161f;
    color: #e6edf3;
  }

  .right-panel-empty {
    color: #687586;
    font-size: 0.76rem;
    line-height: 1.35;
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
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 0.34rem;
    padding: 0.72rem;
    min-width: 0;
    min-height: 0;
    background: #0d1117;
  }

  .note-page {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.28rem;
    min-height: 0;
    color: #d7dde4;
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

  .mode-group {
    display: inline-grid;
    grid-template-columns: repeat(4, auto);
    gap: 0.16rem;
    border: 1px solid #232b36;
    border-radius: 5px;
    background: #0b0f14;
    padding: 0.12rem;
  }

  .mode-button {
    border-color: transparent;
    padding: 0.16rem 0.38rem;
    font-size: 0.7rem;
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

  .body-shell.source-only,
  .body-shell.preview-only {
    grid-template-columns: minmax(0, 1fr);
  }

  .live-editor {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.1rem;
    min-height: 0;
  }

  .live-properties {
    display: grid;
    gap: 0.12rem;
    padding: 0.1rem 0 0.25rem;
  }

  .property-list {
    display: grid;
    gap: 0.08rem;
  }

  .property-row {
    display: grid;
    grid-template-columns: minmax(5.5rem, 0.26fr) minmax(0, 1fr) 1.35rem;
    gap: 0.22rem;
    align-items: center;
    min-height: 1.34rem;
  }

  .property-row input {
    border-color: transparent;
    background: transparent;
    min-height: 1.26rem;
    padding: 0.04rem 0.18rem;
    font-size: 0.8rem;
  }

  .property-row input:first-child {
    color: #8d98a6;
  }

  .property-row input:focus {
    border-color: #303946;
    background: #10161f;
  }

  .add-property-button {
    justify-self: start;
    border-color: transparent;
    background: transparent;
    color: #8d98a6;
    padding: 0.08rem 0.18rem;
    font-size: 0.72rem;
  }

  .add-property-button:hover {
    border-color: #303946;
    background: #10161f;
    color: #d7dde4;
  }

  .property-remove-button {
    display: grid;
    place-items: center;
    width: 1.28rem;
    height: 1.28rem;
    border-color: transparent;
    background: transparent;
    padding: 0;
    color: #5f6b78;
    font-size: 0.68rem;
    opacity: 0;
  }

  .property-row:hover .property-remove-button,
  .property-remove-button:focus {
    opacity: 1;
  }

  .property-remove-button:hover {
    border-color: #303946;
    background: #10161f;
    color: #d7dde4;
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

  .live-body-editor {
    min-height: 0;
    height: 100%;
  }

  .body-shell.source-only .body-editor {
    border-color: transparent;
    border-radius: 0;
    background: transparent;
    padding: 0.12rem 0 0;
    box-shadow: none;
  }

  .body-shell.source-only .body-editor:focus {
    border-color: transparent;
    box-shadow: none;
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
    tab-size: 2;
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
  .markdown-preview :global(li),
  .markdown-preview :global(ul),
  .markdown-preview :global(ol),
  .markdown-preview :global(blockquote),
  .markdown-preview :global(pre) {
    margin: 0 0 0.28rem;
  }

  .markdown-preview :global(p),
  .markdown-preview :global(li) {
    white-space: pre-wrap;
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

  .markdown-preview :global([data-mdp-rule='underline']) {
    display: block;
    height: 0;
    margin: 0.35rem 0;
    border: 0;
    border-top: 2px solid #5d6b7c;
    opacity: 1;
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
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.5rem;
    border-top: 1px solid #232b36;
    padding-top: 0.28rem;
  }

  footer span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 760px) {
    .app-shell {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(0, 1fr) auto;
    }

    .sidebar {
      border-right: 0;
      border-bottom: 1px solid #232b36;
    }

    .panel-resize-handle {
      display: none;
    }

    .right-panel {
      border-top: 1px solid #232b36;
      border-left: 0;
      max-height: 28vh;
    }

  }
</style>
