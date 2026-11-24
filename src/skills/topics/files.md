# Find and edit the shared files

Get the folder paths from the selected project's progress table; never assume or hard-code their location. For general project metadata, use an explicit project:

```sh
agpj --project "<name>" workspace --json
```

Read the `README.md` in the returned `folder` before editing. Then:

- Edit only `epics/<EPIC-ID>/EPIC.md` and the `tasks/<task-id>/TASK.md` of tasks that are not drafts.
- From Doing onward, use `agpj edit` for task bodies and `agpj acceptance check <id> <AC-n>` for verified conditions; do not edit `TASK.md` directly. For direct edits of `EPIC.md` or Backlog and Todo task plans, write a temporary file in the same folder and rename it over the original. Preserve the current history and all unknown frontmatter metadata. The board picks up the change automatically. To record a decision that matters later, use `history add` (see `agpj skills work-loop`) instead of replacing the file just to append history; do not log routine progress.
- Never edit `project.json`, `epic.json`, `task.json`, or `.internal/`. Never delete a task or epic folder.
- Keep machine-specific absolute paths out of shared files.
- The user prepares agent access to the data folder once with `agpj setup`. If a write to the shared folder is refused by the sandbox or by permissions, report it to the user. Do not run `agpj setup` or edit Codex or Claude Code settings yourself.

`edit --desc-file` replaces a task body and acceptance conditions while preserving its latest history and its question references (`questions`); it cannot append history, and `history` or `questions` in the input file are ignored. `epic edit --document-file` replaces the whole `EPIC.md` text. Task bodies are limited to 50,000 characters and `EPIC.md` text to 100,000; if a task plan would exceed its limit, split it into more tasks after asking the user.

A task's completion conditions live in its frontmatter (`acceptance`), and the board reads them. Epic reference conditions belong in the free Markdown body of `EPIC.md` and are not managed by the app. Start from the templates in `agpj skills epic-template` and `agpj skills task-template` (the task template recommends body headings), and read `agpj skills file-format` before writing either file.
