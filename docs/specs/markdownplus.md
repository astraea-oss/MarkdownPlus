# MarkdownPlus Format

MarkdownPlus files use the `.mdp` extension. The first implementation treats them as YAML frontmatter followed by a Markdown-compatible body.

```md
---
id: "018ff6e2-9f4b-7a64-b101-0e2fd6e32f20"
title: "Example note"
created_at: "2026-06-28T23:30:00Z"
updated_at: "2026-06-28T23:30:00Z"
tags: []
aliases: []
type: "note"
---

Markdown body.
```

Required fields:

- `id`
- `title`
- `created_at`
- `updated_at`

## Body Rendering Rules

MarkdownPlus deliberately differs from regular Markdown in these first rules:

- A standalone `---` line renders only as an underline/divider.
- `---` never turns the previous line into a Setext heading.
- Single line breaks are preserved in preview instead of collapsing source lines into one paragraph line.

Use `#`, `##`, and `###` for headings.
