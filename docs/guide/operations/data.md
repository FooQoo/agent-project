---
title: Data and backups
---

# Data and backups

Agent Project keeps everything as plain files in one folder on your machine. This page shows where they are, how projects and folders are registered, how to give agents access to the folder, and how to back it up.

## Where the data lives

The default data folder is `.agent-project/` directly in your home directory:

| OS | Default folder |
| --- | --- |
| macOS | `~/.agent-project/` |
| Windows | `%USERPROFILE%\.agent-project\` |

Set `AGPJ_HOME` to use another data folder. An empty value counts as unset. The CLI, server, hooks, and `agpj setup` must use the same folder to share a board.

```text
.agent-project/
  bindings.json          folder registrations (machine-specific)
  serve.json             how the CLI finds a running agpj serve
  serve.log              output of a server started in the background
  autostart.lock         keeps commands from starting two servers
  autostart.paused       left by agpj stop; the hook does not start a server
  plugin-update/         lock and record of the automatic plugin update, per agent
  .internal/
    preferences.json     language and color theme (written by the app)
  shared/
    workspace.json
    projects/<project-id>/
      README.md
      project.json
      docs/
        README.md
      tasks/<task-id>/
        task.json
        TASK.md
        assets/
      epics/<EPIC-ID>/
        epic.json
        EPIC.md
        assets/
      qa/Q-<n>/
        QA.md
        attachments/
```

The files are plain Markdown and JSON. What each file must contain is described in [Markdown file format](../reference/file-format). A file whose `formatVersion` is not 1 is unsupported: the application reports it and leaves the file unchanged. It never migrates, falls back to, deletes, or resets old data.

The `.internal/` folder holds the app's own settings. `preferences.json` keeps the language and color theme you chose in **Settings**, and it applies to every browser on this PC and to remote access. The app writes this file, so do not edit it by hand. It is outside `shared/`, so it is not part of a project's files.

Each project's `qa/` folder holds the messages: the questions agents asked, with the answers given, and the requests you sent to agents. Each message is one folder such as `qa/Q-12/`, with its text and status in `QA.md` and any images in `attachments/`. Questions and requests share one numbering, so an ID such as `Q-12` is either a question or a request. A task refers to questions from the `questions` list in its `TASK.md`. Read them with `agpj message ls` and `agpj message show`; do not edit `QA.md` by hand. See [Decisions](../delegate/decisions).

Projects that were saved by an earlier build are converted once, the first time the project is read or written. Questions that were still unanswered and not withdrawn keep their numbers; answered and withdrawn ones are dropped. Unanswered start approvals are not carried over: the AI tasks that waited for one move to the Backlog, and a person sends them to Todo again. A folder that cannot be read is moved to `.internal/legacy-qa/`.

A `task.json` or `epic.json` that holds `"draft": "ai"` is an invalid file: the board reports it, leaves it out, and leaves it unchanged.

Each project's `docs/` folder holds your own Markdown documents, which you read and edit from **Docs** in the board header menu. Agent Project creates the folder with a short `README.md` and an empty `docs/release/` folder (an existing project gets `docs/release/` the first time you add or generate a version), but it does not treat the files as board data: changing them never changes the board, with one exception: the `*.md` files directly inside `docs/release/` are the project's list of versions (`v1.8.0.md` lists `v1.8.0`), so adding or removing one changes that list. See [Project docs](../watch/docs).

## Projects and folders

Most commands name the project with `--project NAME`. A project keeps its ID when you rename it, so folder registrations, tasks, and history follow the new name.

```sh
agpj project create "My project"
agpj project ls --json
agpj --project "My project" project rename "Website launch"
```

Update the name in commands that pass `--project` explicitly; folder-based progress and hooks keep resolving the same project. `project rm` deletes the named project, its tasks, and its `docs/` folder, asks for a person's confirmation, and is unavailable to agents.

### Register a folder

`agpj progress` and hooks use the nearest registered project folder when you do not pass `--project`. `project create` also registers the current working folder. Use `--folder` for another existing folder, or `project bind` for an existing project:

```sh
agpj project create "Website launch" --folder ./website
agpj project bind "My project"
agpj project bind "My project" --folder ./docs
```

A folder belongs to one project; several folders can belong to the same project. Commands in subfolders use the nearest registered ancestor. A missing or broken registration returns an error instead of guessing another project; other commands never infer a project from the browser or from another agent.

Registrations live separately in `bindings.json` in the data folder and contain machine-specific paths. Keep that file for a same-machine backup if you want to; use `project bind` to register the right folders on another machine. Do not commit it or place it inside the shared project files.

## Set up the shared folder

`agpj setup` prepares the data folder for Codex and Claude Code, and installs the Agent Project plugin (see [Working with agents](../delegate/agents#set-up-your-agents)). It needs no project. For the data folder it does the following:

- creates the data folder and its `shared/` folder;
- adds the data folder to `sandbox_workspace_write.writable_roots` in Codex's `config.toml`, in `$CODEX_HOME` or `~/.codex`;
- writes `rules/agpj.rules` in the same Codex folder, so that Codex runs `agpj` without asking, unless the file exists already;
- adds the data folder to `permissions.additionalDirectories` in Claude Code's `settings.json`, in `$CLAUDE_CONFIG_DIR` or `~/.claude`.

```sh
agpj setup --agent all --dry-run
agpj setup
```

`--dry-run` prints the planned changes and changes nothing. `--json` prints the same result as machine-readable JSON. A dry run prints output like this (`<home>` stands for your home directory, and `X.Y.Z` for the version):

```text
Dry run: nothing was installed, created, or changed.
Data directory: <home>/.agent-project
Shared folder: <home>/.agent-project/shared
Codex: planned
  plugin: install version X.Y.Z from https://github.com/FooQoo/agent-project.git at vX.Y.Z
  planned <home>/.codex/config.toml
    - add "<home>/.agent-project" to sandbox_workspace_write.writable_roots
  planned <home>/.codex/rules/agpj.rules
    - allow the agpj command (prefix_rule)
Claude Code: planned
  plugin: install version X.Y.Z from https://github.com/FooQoo/agent-project.git at vX.Y.Z
  planned <home>/.claude/settings.json
    - add "<home>/.agent-project" to permissions.additionalDirectories
```

Setup keeps every existing setting and changes only the arrays above. Before it changes an existing settings file, it writes a backup next to that file, named `<file>.agpj-backup…`. The backup holds your original settings, so keep it out of shared folders. Running setup again changes nothing, and each file is reported as `unchanged`. If a settings file cannot be read or has the wrong shape, setup writes nothing at all and names the file.

Setup changes Codex's `config.toml` only when `sandbox_mode` is `"workspace-write"` and no permission profile (`default_permissions`, `permissions`, `profile`, or `profiles`) is set. Otherwise it leaves the file as it is and shows the manual step; the plugin is still installed. Setup never changes `sandbox_mode` itself. With `sandbox_mode = "danger-full-access"`, Codex already reaches every folder, so setup writes neither the permission nor the rule file.

Setup is a step that you run; do not ask an agent to run it during normal work. Project or managed settings can still limit access to the folder, so confirm it in a new session.

## Back up and restore

Stop the server with `agpj stop`, and stop agents and editors writing to the files, before backing up or restoring. Copy the whole `shared/` folder to preserve all projects and the workspace format. For one project, copy its entire folder, using the `folder` value from `workspace --json`. Restore the copy to the same location before restarting the server and agents.

Copying `shared/` does not include `.internal/`, so a project backup does not carry the language and color theme. To keep them too, copy `preferences.json` as well.

## Next

- [Working with agents](../delegate/agents): set up an agent to use this folder.
- [Troubleshooting](./troubleshooting): a project or folder that cannot be found.
