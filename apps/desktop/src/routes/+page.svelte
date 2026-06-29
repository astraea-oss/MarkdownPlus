<script lang="ts">
  import { onMount } from 'svelte';
  import {
    FileText,
    FolderOpen,
    ListTree,
    Plus,
    Save,
    Settings,
    SlidersHorizontal,
    X
  } from '@lucide/svelte';
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
    selectWorkspaceDirectory,
    updateAppSettings
  } from '$lib/api';
  import type {
    DockSide,
    NoteSource,
    NoteSummary,
    PanelMode,
    ToolAnchor,
    WorkspaceSummary
  } from '$lib/types';

  type EditorMode = 'live' | 'source' | 'split' | 'preview';
  type RibbonToolId = 'notes' | 'new-note' | 'settings' | 'outline';
  type RibbonTool = {
    id: RibbonToolId;
    label: string;
    dock: DockSide;
    anchor: ToolAnchor;
    order: number;
  };
  type ToolDocks = Record<RibbonToolId, DockSide>;
  type ToolAnchors = Record<RibbonToolId, ToolAnchor>;
  type ToolOrders = Record<RibbonToolId, number>;
  type ToolZone = {
    anchor: ToolAnchor;
    tools: RibbonTool[];
  };
  type HudToolId = 'notes' | 'outline';
  type HudHeights = Record<HudToolId, number>;
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
  const minRightPanelWidth = 210;
  const maxRightPanelWidth = 420;
  const ribbonPanelWidth = 46;
  const minHudHeight = 96;
  const maxHudHeight = 720;
  const defaultToolDocks: ToolDocks = {
    notes: 'left',
    'new-note': 'left',
    settings: 'left',
    outline: 'right'
  };
  const defaultToolAnchors: ToolAnchors = {
    notes: 'top',
    'new-note': 'top',
    settings: 'bottom',
    outline: 'top'
  };
  const defaultToolOrders: ToolOrders = {
    notes: 10,
    'new-note': 20,
    settings: 30,
    outline: 40
  };
  const defaultHudHeights: HudHeights = {
    notes: 320,
    outline: 190
  };

  let leftPanelWidth = 285;
  let rightPanelWidth = 255;
  let leftPanelMode: PanelMode = 'view';
  let rightPanelMode: PanelMode = 'view';
  let toolDocks: ToolDocks = { ...defaultToolDocks };
  let toolAnchors: ToolAnchors = { ...defaultToolAnchors };
  let toolOrders: ToolOrders = { ...defaultToolOrders };
  let draggingTool: RibbonToolId | null = null;
  let hudHeights: HudHeights = { ...defaultHudHeights };
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
  $: leftPanelColumnWidth = leftPanelMode === 'ribbon' ? ribbonPanelWidth : leftPanelWidth;
  $: rightPanelColumnWidth = rightPanelMode === 'ribbon' ? ribbonPanelWidth : rightPanelWidth;
  $: ribbonTools = buildRibbonTools(toolDocks, toolAnchors, toolOrders);
  $: leftRibbonTools = ribbonTools.filter((tool) => tool.dock === 'left');
  $: rightRibbonTools = ribbonTools.filter((tool) => tool.dock === 'right');
  $: leftToolZones = buildToolZones(leftRibbonTools);
  $: rightToolZones = buildToolZones(rightRibbonTools);
  $: leftHasHudTool = hasHudTool(leftRibbonTools);
  $: rightHasHudTool = hasHudTool(rightRibbonTools);
  $: outlineItems = extractOutline(extractMarkdownBody(noteSource));
  $: markdownHtml = DOMPurify.sanitize(
    marked.parse(markdownPlusPreviewSource(extractMarkdownBody(noteSource)), {
      async: false,
      breaks: true
    }) as string
  );

  onMount(async () => {
    const settings = await getAppSettings();
    applyPersistedSettings(settings);

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
    if (leftPanelMode === 'ribbon') return;
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
      persistUiSettingsInBackground();
    };

    window.addEventListener('pointermove', resizePanel);
    window.addEventListener('pointerup', stopResize);
    window.addEventListener('pointercancel', stopResize);
  }

  function handleLeftPanelResizeKeydown(event: KeyboardEvent) {
    if (leftPanelMode === 'ribbon') return;

    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      leftPanelWidth = clamp(leftPanelWidth - 12, minLeftPanelWidth, maxLeftPanelWidth);
      persistUiSettingsInBackground();
    }

    if (event.key === 'ArrowRight') {
      event.preventDefault();
      leftPanelWidth = clamp(leftPanelWidth + 12, minLeftPanelWidth, maxLeftPanelWidth);
      persistUiSettingsInBackground();
    }
  }

  function startRightPanelResize(event: PointerEvent) {
    if (rightPanelMode === 'ribbon') return;
    event.preventDefault();

    const startX = event.clientX;
    const startWidth = rightPanelWidth;
    document.body.classList.add('is-resizing-panel');

    const resizePanel = (moveEvent: PointerEvent) => {
      rightPanelWidth = clamp(startWidth - (moveEvent.clientX - startX), minRightPanelWidth, maxRightPanelWidth);
    };

    const stopResize = () => {
      document.body.classList.remove('is-resizing-panel');
      window.removeEventListener('pointermove', resizePanel);
      window.removeEventListener('pointerup', stopResize);
      window.removeEventListener('pointercancel', stopResize);
      persistUiSettingsInBackground();
    };

    window.addEventListener('pointermove', resizePanel);
    window.addEventListener('pointerup', stopResize);
    window.addEventListener('pointercancel', stopResize);
  }

  function handleRightPanelResizeKeydown(event: KeyboardEvent) {
    if (rightPanelMode === 'ribbon') return;

    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      rightPanelWidth = clamp(rightPanelWidth + 12, minRightPanelWidth, maxRightPanelWidth);
      persistUiSettingsInBackground();
    }

    if (event.key === 'ArrowRight') {
      event.preventDefault();
      rightPanelWidth = clamp(rightPanelWidth - 12, minRightPanelWidth, maxRightPanelWidth);
      persistUiSettingsInBackground();
    }
  }

  function startHudResize(event: PointerEvent, tool: HudToolId) {
    event.preventDefault();

    const startY = event.clientY;
    const startHeight = hudHeights[tool];
    document.body.classList.add('is-resizing-hud');

    const resizeHud = (moveEvent: PointerEvent) => {
      setHudHeight(tool, startHeight + moveEvent.clientY - startY);
    };

    const stopResize = () => {
      document.body.classList.remove('is-resizing-hud');
      window.removeEventListener('pointermove', resizeHud);
      window.removeEventListener('pointerup', stopResize);
      window.removeEventListener('pointercancel', stopResize);
      persistUiSettingsInBackground();
    };

    window.addEventListener('pointermove', resizeHud);
    window.addEventListener('pointerup', stopResize);
    window.addEventListener('pointercancel', stopResize);
  }

  function handleHudResizeKeydown(event: KeyboardEvent, tool: HudToolId) {
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      setHudHeight(tool, hudHeights[tool] - 16);
      persistUiSettingsInBackground();
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setHudHeight(tool, hudHeights[tool] + 16);
      persistUiSettingsInBackground();
    }
  }

  function setHudHeight(tool: HudToolId, height: number) {
    hudHeights = {
      ...hudHeights,
      [tool]: clamp(Math.round(height), minHudHeight, maxHudHeight)
    };
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function normalizePanelMode(mode: string): PanelMode {
    return mode === 'ribbon' ? 'ribbon' : 'view';
  }

  function normalizeToolAnchor(anchor: string): ToolAnchor {
    if (anchor === 'center' || anchor === 'bottom') return anchor;
    return 'top';
  }

  function normalizeDockSide(side: string): DockSide {
    return side === 'right' ? 'right' : 'left';
  }

  function applyPersistedSettings(settings: Awaited<ReturnType<typeof getAppSettings>>) {
    portableRoot = settings.portable_root;
    leftPanelWidth = clamp(settings.left_panel_width, minLeftPanelWidth, maxLeftPanelWidth);
    rightPanelWidth = clamp(settings.right_panel_width, minRightPanelWidth, maxRightPanelWidth);
    leftPanelMode = normalizePanelMode(settings.left_panel_mode);
    rightPanelMode = normalizePanelMode(settings.right_panel_mode);
    toolDocks = {
      notes: normalizeDockSide(settings.notes_dock),
      'new-note': normalizeDockSide(settings.new_note_dock),
      settings: normalizeDockSide(settings.settings_dock),
      outline: normalizeDockSide(settings.outline_dock)
    };
    toolAnchors = {
      notes: normalizeToolAnchor(settings.notes_anchor),
      'new-note': normalizeToolAnchor(settings.new_note_anchor),
      settings: normalizeToolAnchor(settings.settings_anchor),
      outline: normalizeToolAnchor(settings.outline_anchor)
    };
    toolOrders = {
      notes: settings.notes_order,
      'new-note': settings.new_note_order,
      settings: settings.settings_order,
      outline: settings.outline_order
    };
    hudHeights = {
      notes: clamp(settings.notes_hud_height, minHudHeight, maxHudHeight),
      outline: clamp(settings.outline_hud_height, minHudHeight, maxHudHeight)
    };
  }

  function buildRibbonTools(docks: ToolDocks, anchors: ToolAnchors, orders: ToolOrders): RibbonTool[] {
    return [
      { id: 'notes', label: 'Notes', dock: docks.notes, anchor: anchors.notes, order: orders.notes },
      { id: 'new-note', label: 'New note', dock: docks['new-note'], anchor: anchors['new-note'], order: orders['new-note'] },
      { id: 'settings', label: 'Settings', dock: docks.settings, anchor: anchors.settings, order: orders.settings },
      { id: 'outline', label: 'Outline', dock: docks.outline, anchor: anchors.outline, order: orders.outline }
    ];
  }

  function buildToolZones(tools: RibbonTool[]): ToolZone[] {
    return [
      { anchor: 'top', tools: sortTools(tools.filter((tool) => tool.anchor === 'top')) },
      { anchor: 'center', tools: sortTools(tools.filter((tool) => tool.anchor === 'center')) },
      { anchor: 'bottom', tools: sortTools(tools.filter((tool) => tool.anchor === 'bottom')) }
    ];
  }

  function sortTools(tools: RibbonTool[]): RibbonTool[] {
    return [...tools].sort((first, second) => first.order - second.order || first.label.localeCompare(second.label));
  }

  async function setPanelMode(panel: 'left' | 'right', mode: PanelMode) {
    const previousLeftPanelMode = leftPanelMode;
    const previousRightPanelMode = rightPanelMode;

    if (panel === 'left') {
      leftPanelMode = mode;
    } else {
      rightPanelMode = mode;
    }

    try {
      await persistUiSettings();
    } catch (error) {
      leftPanelMode = previousLeftPanelMode;
      rightPanelMode = previousRightPanelMode;
      status = error instanceof Error ? error.message : String(error);
    }
  }

  async function setToolDock(tool: RibbonToolId, side: DockSide) {
    await moveTool(tool, side, toolAnchors[tool], undefined, 'append', `${toolLabel(tool)} moved to ${side} panel.`);
  }

  async function setToolAnchor(tool: RibbonToolId, anchor: ToolAnchor) {
    await moveTool(tool, toolDocks[tool], anchor, undefined, 'append', `${toolLabel(tool)} anchored ${anchor}.`);
  }

  async function moveToolToOppositeDock(tool: RibbonToolId) {
    const currentDock = getToolDock(tool);
    await setToolDock(tool, currentDock === 'left' ? 'right' : 'left');
  }

  function getToolDock(tool: RibbonToolId): DockSide {
    return toolDocks[tool];
  }

  function hasHudTool(tools: RibbonTool[]): boolean {
    return tools.some((tool) => tool.id === 'notes' || tool.id === 'outline');
  }

  function toolLabel(tool: RibbonToolId): string {
    if (tool === 'new-note') return 'New note';
    return tool.charAt(0).toUpperCase() + tool.slice(1);
  }

  function handleRibbonDragStart(event: DragEvent, tool: RibbonToolId) {
    draggingTool = tool;
    event.dataTransfer?.setData('text/plain', tool);
    event.dataTransfer?.setData('application/x-markdownplus-ribbon-tool', tool);
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function handleRibbonDragEnd() {
    draggingTool = null;
  }

  function allowRibbonDrop(event: DragEvent) {
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
  }

  async function handleRibbonDrop(event: DragEvent, side: DockSide) {
    event.preventDefault();
    event.stopPropagation();
    const tool = event.dataTransfer?.getData('application/x-markdownplus-ribbon-tool')
      || event.dataTransfer?.getData('text/plain');

    if (isRibbonToolId(tool)) {
      await moveTool(tool, side, toolAnchors[tool], undefined, 'append', `${toolLabel(tool)} moved to ${side} panel.`);
    }
  }

  async function handleToolZoneDrop(event: DragEvent, side: DockSide, anchor: ToolAnchor) {
    event.preventDefault();
    event.stopPropagation();
    const tool = getDraggedTool(event);

    if (tool) {
      await moveTool(tool, side, anchor, undefined, 'append');
    }
  }

  async function handleToolDrop(event: DragEvent, side: DockSide, anchor: ToolAnchor, targetTool: RibbonToolId) {
    event.preventDefault();
    event.stopPropagation();
    const tool = getDraggedTool(event);

    if (tool) {
      if (tool === targetTool) return;

      const target = event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
      const placement = target && event.clientY > target.getBoundingClientRect().top + target.getBoundingClientRect().height / 2
        ? 'after'
        : 'before';
      await moveTool(tool, side, anchor, targetTool, placement);
    }
  }

  function getDraggedTool(event: DragEvent): RibbonToolId | null {
    const tool = event.dataTransfer?.getData('application/x-markdownplus-ribbon-tool')
      || event.dataTransfer?.getData('text/plain');

    return isRibbonToolId(tool) ? tool : null;
  }

  async function moveTool(
    tool: RibbonToolId,
    side: DockSide,
    anchor: ToolAnchor,
    targetTool: RibbonToolId | undefined,
    placement: 'before' | 'after' | 'append',
    successStatus = `${toolLabel(tool)} moved to ${anchor} ${side}.`
  ) {
    const previousToolDocks = toolDocks;
    const previousToolAnchors = toolAnchors;
    const previousToolOrders = toolOrders;
    const targetTools = sortTools(ribbonTools)
      .filter((candidate) => candidate.dock === side && candidate.anchor === anchor && candidate.id !== tool)
      .map((candidate) => candidate.id);

    if (targetTool && targetTool !== tool) {
      const targetIndex = targetTools.indexOf(targetTool);
      const insertAt = targetIndex === -1
        ? targetTools.length
        : targetIndex + (placement === 'after' ? 1 : 0);
      targetTools.splice(insertAt, 0, tool);
    } else {
      targetTools.push(tool);
    }

    toolDocks = { ...toolDocks, [tool]: side };
    toolAnchors = { ...toolAnchors, [tool]: anchor };
    toolOrders = { ...toolOrders, ...renumberTools(targetTools) };

    try {
      await persistUiSettings();
      status = successStatus;
    } catch (error) {
      toolDocks = previousToolDocks;
      toolAnchors = previousToolAnchors;
      toolOrders = previousToolOrders;
      status = error instanceof Error ? error.message : String(error);
    }
  }

  function renumberTools(toolIds: RibbonToolId[]): Partial<ToolOrders> {
    return Object.fromEntries(toolIds.map((tool, index) => [tool, (index + 1) * 10])) as Partial<ToolOrders>;
  }

  function isRibbonToolId(value: string | undefined): value is RibbonToolId {
    return value === 'notes'
      || value === 'new-note'
      || value === 'settings'
      || value === 'outline';
  }

  function runRibbonTool(tool: RibbonToolId) {
    if (tool === 'notes') {
      settingsOpen = false;
      return;
    }

    if (tool === 'new-note') {
      if (!workspace) {
        status = 'Open a workspace first.';
        return;
      }

      createNewNote();
      return;
    }

    if (tool === 'settings') {
      settingsOpen = true;
    }
  }

  async function persistUiSettings() {
    const settings = await updateAppSettings({
      left_panel_width: Math.round(leftPanelWidth),
      right_panel_width: Math.round(rightPanelWidth),
      left_panel_mode: leftPanelMode,
      right_panel_mode: rightPanelMode,
      notes_dock: toolDocks.notes,
      new_note_dock: toolDocks['new-note'],
      settings_dock: toolDocks.settings,
      outline_dock: toolDocks.outline,
      notes_anchor: toolAnchors.notes,
      new_note_anchor: toolAnchors['new-note'],
      settings_anchor: toolAnchors.settings,
      outline_anchor: toolAnchors.outline,
      notes_order: toolOrders.notes,
      new_note_order: toolOrders['new-note'],
      settings_order: toolOrders.settings,
      outline_order: toolOrders.outline,
      notes_hud_height: hudHeights.notes,
      outline_hud_height: hudHeights.outline
    });
    applyPersistedSettings(settings);
  }

  function persistUiSettingsInBackground() {
    void persistUiSettings().catch((error) => {
      status = error instanceof Error ? error.message : String(error);
    });
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

<main
  class="app-shell"
  style={`--left-panel-width: ${leftPanelColumnWidth}px; --right-panel-width: ${rightPanelColumnWidth}px;`}
>
  <aside
    class:ribbon-panel={leftPanelMode === 'ribbon'}
    class="sidebar"
    on:dragover={allowRibbonDrop}
    on:drop={(event) => handleRibbonDrop(event, 'left')}
  >
    {#if leftPanelMode === 'ribbon'}
      <nav
        class="panel-ribbon"
        aria-label="Left ribbon"
        on:dragover={allowRibbonDrop}
        on:drop={(event) => handleRibbonDrop(event, 'left')}
      >
        {#each leftToolZones as zone}
          <div
            class:center-zone={zone.anchor === 'center'}
            class:bottom-zone={zone.anchor === 'bottom'}
            class:drop-ready={Boolean(draggingTool)}
            class="tool-zone"
            role="group"
            aria-label={`Left ${zone.anchor} tools`}
            on:dragover={allowRibbonDrop}
            on:drop={(event) => handleToolZoneDrop(event, 'left', zone.anchor)}
          >
            {#each zone.tools as tool}
              <button
                class:active={tool.id === 'settings' && settingsOpen}
                class:drag-enabled={true}
                class="ribbon-button"
                aria-label={tool.label}
                aria-disabled={tool.id === 'new-note' && !workspace}
                title={`${tool.label} - drag to move`}
                draggable="true"
                on:click={() => runRibbonTool(tool.id)}
                on:dblclick={() => moveToolToOppositeDock(tool.id)}
                on:dragstart={(event) => handleRibbonDragStart(event, tool.id)}
                on:dragend={handleRibbonDragEnd}
                on:dragover={allowRibbonDrop}
                on:drop={(event) => handleToolDrop(event, 'left', zone.anchor, tool.id)}
              >
                {#if tool.id === 'notes'}
                  <FileText size={17} />
                {:else if tool.id === 'new-note'}
                  <Plus size={17} />
                {:else if tool.id === 'settings'}
                  <Settings size={17} />
                {:else if tool.id === 'outline'}
                  <ListTree size={17} />
                {/if}
              </button>
            {/each}
          </div>
        {/each}
      </nav>
    {:else}
      <div class="brand" data-tauri-drag-region>
        <div>
          <h1>MarkdownPlus</h1>
          <p>Local .mdp workspace</p>
        </div>
      </div>

      <div class="panel-tool-stack" aria-label="Left panel tools">
        {#each leftToolZones as zone}
          <div
            class:center-zone={zone.anchor === 'center'}
            class:bottom-zone={zone.anchor === 'bottom'}
            class:drop-ready={Boolean(draggingTool)}
            class="tool-zone"
            role="group"
            aria-label={`Left ${zone.anchor} tools`}
            on:dragover={allowRibbonDrop}
            on:drop={(event) => handleToolZoneDrop(event, 'left', zone.anchor)}
          >
            {#each zone.tools as tool}
              <section
                class="panel-tool-group"
                role="group"
                aria-label={tool.label}
                draggable="true"
                on:dragstart={(event) => handleRibbonDragStart(event, tool.id)}
                on:dragend={handleRibbonDragEnd}
                on:dragover={allowRibbonDrop}
                on:drop={(event) => handleToolDrop(event, 'left', zone.anchor, tool.id)}
              >
            <div class:has-compact-action={tool.id === 'notes'} class="panel-tool-row">
              <button
                class:active={tool.id === 'settings' && settingsOpen}
                class:drag-enabled={true}
                class="panel-tool-button"
                aria-label={tool.label}
                aria-disabled={tool.id === 'new-note' && !workspace}
                title={`${tool.label} - drag to move`}
                draggable="true"
                on:click={() => runRibbonTool(tool.id)}
                on:dblclick={() => moveToolToOppositeDock(tool.id)}
                on:dragstart={(event) => handleRibbonDragStart(event, tool.id)}
                on:dragend={handleRibbonDragEnd}
              >
                {#if tool.id === 'notes'}
                  <FileText size={15} />
                {:else if tool.id === 'new-note'}
                  <Plus size={15} />
                {:else if tool.id === 'settings'}
                  <Settings size={15} />
                {:else if tool.id === 'outline'}
                  <ListTree size={15} />
                {/if}
                <span>{tool.label}</span>
              </button>
              {#if tool.id === 'notes'}
                <button class="compact-action" disabled={!workspace} on:click={createNewNote}>New</button>
              {/if}
            </div>

            {#if tool.id === 'notes'}
              <div class="tool-hud notes-hud" style={`height: ${hudHeights.notes}px;`}>
                {#if workspace}
                  <nav class="notes-list" aria-label="Notes">
                    {#each notes as note}
                      <button
                        class:active={note.id === selectedId}
                        class="note-row"
                        on:click={() => {
                          settingsOpen = false;
                          selectNote(note.id);
                        }}
                      >
                        <span class="note-title">{note.title}</span>
                      </button>
                    {/each}
                  </nav>
                  <div class="notes-count">{notes.length} notes</div>
                {:else}
                  <div class="sidebar-empty">Open Settings to choose a workspace.</div>
                {/if}
              </div>
              <button
                class="hud-resize-handle"
                type="button"
                aria-label="Resize Notes HUD"
                on:pointerdown={(event) => startHudResize(event, 'notes')}
                on:keydown={(event) => handleHudResizeKeydown(event, 'notes')}
              ></button>
            {:else if tool.id === 'outline'}
              <div class="tool-hud outline-hud" style={`height: ${hudHeights.outline}px;`}>
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
              </div>
              <button
                class="hud-resize-handle"
                type="button"
                aria-label="Resize Outline HUD"
                on:pointerdown={(event) => startHudResize(event, 'outline')}
                on:keydown={(event) => handleHudResizeKeydown(event, 'outline')}
              ></button>
            {/if}
              </section>
            {/each}
          </div>
        {/each}

        {#if !leftHasHudTool}
          <div class="sidebar-empty">
            {workspace ? 'Drop tools here.' : 'Open Settings to choose a workspace.'}
          </div>
        {/if}
      </div>

    {/if}
  </aside>

  <button
    class="panel-resize-handle"
    type="button"
    aria-label="Resize left panel"
    disabled={leftPanelMode === 'ribbon'}
    on:pointerdown={startLeftPanelResize}
    on:keydown={handleLeftPanelResizeKeydown}
  ></button>

  <section class="editor">
    {#if settingsOpen}
      <div class="settings-view">
        <header class="settings-view-header" data-tauri-drag-region>
          <div>
            <h2>Settings</h2>
            <p>MarkdownPlusData</p>
          </div>
          <button class="icon-button" aria-label="Close settings" on:click={() => (settingsOpen = false)}>
            <X size={15} />
          </button>
        </header>

        <div class="settings-view-content">
          <section class="settings-section">
            <div class="settings-section-heading">
              <FolderOpen size={16} />
              <h3>Workspace</h3>
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

          <section class="settings-section">
            <div class="settings-section-heading">
              <SlidersHorizontal size={16} />
              <h3>Panels</h3>
            </div>

            <div class="setting-row">
              <div>
                <span>Left panel</span>
                <p>{leftPanelMode === 'ribbon' ? 'Ribbon' : `${leftPanelWidth}px`}</p>
              </div>
              <div class="segmented-control" aria-label="Left panel mode">
                <button class:active={leftPanelMode === 'view'} on:click={() => setPanelMode('left', 'view')}>View</button>
                <button class:active={leftPanelMode === 'ribbon'} on:click={() => setPanelMode('left', 'ribbon')}>Ribbon</button>
              </div>
            </div>

            <div class="setting-row">
              <div>
                <span>Right panel</span>
                <p>{rightPanelMode === 'ribbon' ? 'Ribbon' : `${rightPanelWidth}px`}</p>
              </div>
              <div class="segmented-control" aria-label="Right panel mode">
                <button class:active={rightPanelMode === 'view'} on:click={() => setPanelMode('right', 'view')}>View</button>
                <button class:active={rightPanelMode === 'ribbon'} on:click={() => setPanelMode('right', 'ribbon')}>Ribbon</button>
              </div>
            </div>

            <div class="settings-subsection">
              <h4>Panel tools</h4>
              {#each ribbonTools as tool}
                <div class="setting-row">
                  <div>
                    <span>{tool.label}</span>
                    <p>{tool.dock === 'left' ? 'Left panel' : 'Right panel'} · {tool.anchor}</p>
                  </div>
                  <div class="setting-actions">
                    <div class="segmented-control" aria-label={`${tool.label} dock`}>
                      <button class:active={tool.dock === 'left'} on:click={() => setToolDock(tool.id, 'left')}>Left</button>
                      <button class:active={tool.dock === 'right'} on:click={() => setToolDock(tool.id, 'right')}>Right</button>
                    </div>
                    <div class="segmented-control three-options" aria-label={`${tool.label} anchor`}>
                      <button class:active={tool.anchor === 'top'} on:click={() => setToolAnchor(tool.id, 'top')}>Top</button>
                      <button class:active={tool.anchor === 'center'} on:click={() => setToolAnchor(tool.id, 'center')}>Center</button>
                      <button class:active={tool.anchor === 'bottom'} on:click={() => setToolAnchor(tool.id, 'bottom')}>Bottom</button>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </section>
        </div>
      </div>
    {:else if selectedNoteSource}
      <div class="note-page">
        <header class="editor-header">
          <div>
            <h2>{selectedTitle}</h2>
            <p>{selectedNoteSource.id}</p>
          </div>
          <button class="primary save-button" disabled={saving} on:click={saveSelectedNote}>
            <Save size={15} />
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
      {#if selectedNoteSource && !settingsOpen}
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

  <button
    class="panel-resize-handle right-resize-handle"
    type="button"
    aria-label="Resize right panel"
    disabled={rightPanelMode === 'ribbon'}
    on:pointerdown={startRightPanelResize}
    on:keydown={handleRightPanelResizeKeydown}
  ></button>

  <aside
    class:ribbon-panel={rightPanelMode === 'ribbon'}
    class="right-panel"
    on:dragover={allowRibbonDrop}
    on:drop={(event) => handleRibbonDrop(event, 'right')}
  >
    {#if rightPanelMode === 'ribbon'}
      <nav
        class="panel-ribbon"
        aria-label="Right ribbon"
        on:dragover={allowRibbonDrop}
        on:drop={(event) => handleRibbonDrop(event, 'right')}
      >
        {#each rightToolZones as zone}
          <div
            class:center-zone={zone.anchor === 'center'}
            class:bottom-zone={zone.anchor === 'bottom'}
            class:drop-ready={Boolean(draggingTool)}
            class="tool-zone"
            role="group"
            aria-label={`Right ${zone.anchor} tools`}
            on:dragover={allowRibbonDrop}
            on:drop={(event) => handleToolZoneDrop(event, 'right', zone.anchor)}
          >
            {#each zone.tools as tool}
              <button
                class:active={tool.id === 'settings' && settingsOpen}
                class:drag-enabled={true}
                class="ribbon-button"
                aria-label={tool.label}
                aria-disabled={tool.id === 'new-note' && !workspace}
                title={`${tool.label} - drag to move`}
                draggable="true"
                on:click={() => runRibbonTool(tool.id)}
                on:dblclick={() => moveToolToOppositeDock(tool.id)}
                on:dragstart={(event) => handleRibbonDragStart(event, tool.id)}
                on:dragend={handleRibbonDragEnd}
                on:dragover={allowRibbonDrop}
                on:drop={(event) => handleToolDrop(event, 'right', zone.anchor, tool.id)}
              >
                {#if tool.id === 'notes'}
                  <FileText size={17} />
                {:else if tool.id === 'new-note'}
                  <Plus size={17} />
                {:else if tool.id === 'settings'}
                  <Settings size={17} />
                {:else if tool.id === 'outline'}
                  <ListTree size={17} />
                {/if}
              </button>
            {/each}
          </div>
        {/each}
      </nav>
    {:else}
      <div class="panel-tool-stack" aria-label="Right panel tools">
        {#each rightToolZones as zone}
          <div
            class:center-zone={zone.anchor === 'center'}
            class:bottom-zone={zone.anchor === 'bottom'}
            class:drop-ready={Boolean(draggingTool)}
            class="tool-zone"
            role="group"
            aria-label={`Right ${zone.anchor} tools`}
            on:dragover={allowRibbonDrop}
            on:drop={(event) => handleToolZoneDrop(event, 'right', zone.anchor)}
          >
            {#each zone.tools as tool}
              <section
                class="panel-tool-group"
                role="group"
                aria-label={tool.label}
                draggable="true"
                on:dragstart={(event) => handleRibbonDragStart(event, tool.id)}
                on:dragend={handleRibbonDragEnd}
                on:dragover={allowRibbonDrop}
                on:drop={(event) => handleToolDrop(event, 'right', zone.anchor, tool.id)}
              >
            <div class:has-compact-action={tool.id === 'notes'} class="panel-tool-row">
              <button
                class:active={tool.id === 'settings' && settingsOpen}
                class:drag-enabled={true}
                class="panel-tool-button"
                aria-label={tool.label}
                aria-disabled={tool.id === 'new-note' && !workspace}
                title={`${tool.label} - drag to move`}
                draggable="true"
                on:click={() => runRibbonTool(tool.id)}
                on:dblclick={() => moveToolToOppositeDock(tool.id)}
                on:dragstart={(event) => handleRibbonDragStart(event, tool.id)}
                on:dragend={handleRibbonDragEnd}
              >
                {#if tool.id === 'notes'}
                  <FileText size={15} />
                {:else if tool.id === 'new-note'}
                  <Plus size={15} />
                {:else if tool.id === 'settings'}
                  <Settings size={15} />
                {:else if tool.id === 'outline'}
                  <ListTree size={15} />
                {/if}
                <span>{tool.label}</span>
              </button>
              {#if tool.id === 'notes'}
                <button class="compact-action" disabled={!workspace} on:click={createNewNote}>New</button>
              {/if}
            </div>

            {#if tool.id === 'notes'}
              <div class="tool-hud notes-hud" style={`height: ${hudHeights.notes}px;`}>
                {#if workspace}
                  <nav class="notes-list" aria-label="Notes">
                    {#each notes as note}
                      <button
                        class:active={note.id === selectedId}
                        class="note-row"
                        on:click={() => {
                          settingsOpen = false;
                          selectNote(note.id);
                        }}
                      >
                        <span class="note-title">{note.title}</span>
                      </button>
                    {/each}
                  </nav>
                  <div class="notes-count">{notes.length} notes</div>
                {:else}
                  <div class="sidebar-empty">Open Settings to choose a workspace.</div>
                {/if}
              </div>
              <button
                class="hud-resize-handle"
                type="button"
                aria-label="Resize Notes HUD"
                on:pointerdown={(event) => startHudResize(event, 'notes')}
                on:keydown={(event) => handleHudResizeKeydown(event, 'notes')}
              ></button>
            {:else if tool.id === 'outline'}
              <div class="tool-hud outline-hud" style={`height: ${hudHeights.outline}px;`}>
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
              </div>
              <button
                class="hud-resize-handle"
                type="button"
                aria-label="Resize Outline HUD"
                on:pointerdown={(event) => startHudResize(event, 'outline')}
                on:keydown={(event) => handleHudResizeKeydown(event, 'outline')}
              ></button>
            {/if}
              </section>
            {/each}
          </div>
        {/each}

        {#if !rightHasHudTool}
          <div class="right-panel-empty">
            Drop tools here.
          </div>
        {/if}
      </div>
    {/if}
  </aside>
</main>

<style>
  .app-shell {
    display: grid;
    grid-template-columns: var(--left-panel-width) 4px minmax(0, 1fr) 4px var(--right-panel-width);
    min-height: 100vh;
    max-height: 100vh;
    background: #0d1117;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.72rem;
    background: #0b0f14;
    padding: 0.72rem;
    min-height: 0;
  }

  .sidebar.ribbon-panel,
  .right-panel.ribbon-panel {
    align-items: center;
    gap: 0;
    padding: 0.55rem 0.28rem;
  }

  .panel-ribbon {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    flex: 1 1 auto;
    gap: 0.36rem;
    width: 100%;
    min-height: 0;
  }

  .tool-zone {
    display: grid;
    align-content: start;
    gap: 0.32rem;
    min-height: 1rem;
    width: 100%;
  }

  .tool-zone.drop-ready {
    min-height: 2rem;
    outline: 1px dashed transparent;
    outline-offset: -2px;
  }

  .tool-zone.drop-ready:empty {
    border-radius: 4px;
    outline-color: #23303d;
  }

  .tool-zone.drop-ready:empty:hover {
    outline-color: #2ea987;
  }

  .panel-ribbon .tool-zone {
    justify-items: center;
  }

  .tool-zone.center-zone {
    align-content: center;
  }

  .tool-zone.bottom-zone {
    align-content: end;
  }

  .ribbon-button {
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border-color: transparent;
    background: transparent;
    padding: 0;
    color: #8d98a6;
  }

  .ribbon-button:hover,
  .ribbon-button:focus,
  .ribbon-button.active {
    border-color: #2ea987;
    background: #10211e;
    color: #e6edf3;
  }

  .ribbon-button.drag-enabled {
    cursor: grab;
  }

  .ribbon-button.drag-enabled:active {
    cursor: grabbing;
  }

  .ribbon-button[aria-disabled='true'],
  .panel-tool-button[aria-disabled='true'] {
    opacity: 0.42;
  }

  .panel-tool-stack {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    flex: 1 1 auto;
    min-height: 0;
    gap: 0.38rem;
    overflow: auto;
  }

  .panel-tool-group {
    display: grid;
    gap: 0.16rem;
    min-height: 0;
  }

  .panel-tool-group[draggable='true'] {
    cursor: grab;
  }

  .panel-tool-group[draggable='true']:active {
    cursor: grabbing;
  }

  .panel-tool-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.3rem;
  }

  .panel-tool-row.has-compact-action {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .panel-tool-button {
    display: grid;
    grid-template-columns: 1rem minmax(0, 1fr);
    align-items: center;
    gap: 0.3rem;
    width: 100%;
    border-color: transparent;
    background: transparent;
    padding: 0.28rem 0.32rem;
    color: #aeb8c4;
    font-size: 0.76rem;
    text-align: left;
  }

  .compact-action {
    min-height: 1.45rem;
    padding: 0.18rem 0.38rem;
    font-size: 0.72rem;
    line-height: 1;
  }

  .panel-tool-button span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-tool-button:hover,
  .panel-tool-button:focus,
  .panel-tool-button.active {
    border-color: #2ea987;
    background: #10211e;
    color: #e6edf3;
  }

  .panel-tool-button.drag-enabled {
    cursor: grab;
  }

  .panel-tool-button.drag-enabled:active {
    cursor: grabbing;
  }

  .tool-hud {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.34rem;
    min-height: 0;
    overflow: hidden;
    padding: 0.22rem 0 0.08rem 1.42rem;
  }

  .notes-hud {
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 0.22rem;
    padding-left: 0;
  }

  .outline-hud {
    grid-template-rows: minmax(0, 1fr);
  }

  .hud-resize-handle {
    width: calc(100% - 1.42rem);
    height: 5px;
    min-height: 5px;
    justify-self: end;
    border: 0;
    border-radius: 0;
    background: linear-gradient(to bottom, transparent 0 2px, #232b36 2px 3px, transparent 3px 5px);
    cursor: row-resize;
    padding: 0;
  }

  .hud-resize-handle:hover,
  .hud-resize-handle:focus {
    background: linear-gradient(to bottom, transparent 0 2px, #2ea987 2px 3px, transparent 3px 5px);
    outline: none;
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

  .panel-resize-handle:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .panel-resize-handle:disabled:hover,
  .panel-resize-handle:disabled:focus {
    border-left-color: transparent;
    border-right-color: #232b36;
  }

  .right-resize-handle {
    border-right: 1px solid transparent;
    border-left: 1px solid #232b36;
  }

  .right-resize-handle:disabled:hover,
  .right-resize-handle:disabled:focus {
    border-right-color: transparent;
    border-left-color: #232b36;
  }

  :global(body.is-resizing-panel),
  :global(body.is-resizing-panel *) {
    cursor: col-resize !important;
    user-select: none !important;
  }

  :global(body.is-resizing-hud),
  :global(body.is-resizing-hud *) {
    cursor: row-resize !important;
    user-select: none !important;
  }

  .right-panel {
    display: flex;
    flex-direction: column;
    gap: 0.54rem;
    border-left: 1px solid #232b36;
    background: #0b0f14;
    padding: 0.72rem 0.64rem;
    min-width: 0;
    min-height: 0;
  }

  .outline-list {
    display: grid;
    align-content: start;
    flex: 1 1 auto;
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
    display: grid;
    align-content: start;
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
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 0.38rem;
  }

  .notes-list {
    display: grid;
    align-content: start;
    flex: 1 1 auto;
    gap: 0.12rem;
    overflow: auto;
    min-height: 0;
  }

  .notes-count {
    overflow: hidden;
    color: #7d8896;
    font-size: 0.7rem;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sidebar-empty {
    display: grid;
    align-content: start;
    color: #7d8896;
    font-size: 0.78rem;
    line-height: 1.35;
  }

  .settings-view {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 1rem;
    min-height: 0;
    color: #d7dde4;
  }

  .settings-view-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    gap: 0.72rem;
  }

  .settings-view-header h2 {
    margin: 0;
    color: #f0f4f8;
    font-size: 1.16rem;
    line-height: 1.2;
  }

  .settings-view-header p,
  .settings-section p {
    margin: 0.14rem 0 0;
    color: #7d8896;
    font-size: 0.75rem;
  }

  .settings-view-content {
    display: grid;
    align-content: start;
    gap: 1rem;
    overflow: auto;
    min-height: 0;
    max-width: 58rem;
  }

  .settings-section {
    display: grid;
    gap: 0.7rem;
    border-top: 1px solid #232b36;
    padding-top: 0.72rem;
  }

  .settings-section-heading {
    display: flex;
    align-items: center;
    gap: 0.42rem;
    color: #aeb8c4;
  }

  .settings-section-heading h3 {
    margin: 0;
    color: #e6edf3;
    font-size: 0.88rem;
  }

  .settings-subsection {
    display: grid;
    gap: 0.46rem;
    border-top: 1px solid #1b222c;
    padding-top: 0.62rem;
  }

  .settings-subsection h4 {
    margin: 0;
    color: #d7dde4;
    font-size: 0.8rem;
  }

  .settings-section label,
  .setting-row span {
    color: #9aa6b2;
    font-size: 0.75rem;
    font-weight: 650;
  }

  .setting-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 1rem;
    min-height: 2.35rem;
  }

  .setting-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: end;
    gap: 0.34rem;
  }

  .segmented-control {
    display: inline-grid;
    grid-template-columns: repeat(2, auto);
    gap: 0.16rem;
    border: 1px solid #232b36;
    border-radius: 5px;
    background: #0b0f14;
    padding: 0.12rem;
  }

  .segmented-control.three-options {
    grid-template-columns: repeat(3, auto);
  }

  .segmented-control button {
    border-color: transparent;
    padding: 0.18rem 0.48rem;
    font-size: 0.72rem;
  }

  .segmented-control button.active {
    border-color: #2ea987;
    background: #10211e;
    color: #e6edf3;
  }

  .settings-note {
    overflow: hidden;
    margin: 0;
    color: #7d8896;
    font-size: 0.68rem;
    line-height: 1.3;
    text-overflow: ellipsis;
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
    width: 100%;
    text-align: left;
    background: #0f141b;
    border-color: transparent;
    min-height: 1.55rem;
    padding: 0.24rem 0.38rem;
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
    font-size: 0.78rem;
    font-weight: 650;
    line-height: 1.15;
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

  .save-button {
    display: inline-flex;
    align-items: center;
    gap: 0.34rem;
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
