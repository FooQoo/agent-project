---
title: CLI reference
---

# CLI reference

Use the browser to review and edit work, and `agpj` for agent updates and project management. Most commands take an explicit `--project NAME`; the examples use `My project`, so replace it with your project's name and use the task IDs the CLI returns. Run `agpj --help` or `agpj <command> --help` for every option.

## Commands at a glance

| I want to… | Command |
| --- | --- |
| Add a task | `add` |
| Read a task or the board | `show`, `ls`, `snapshot` |
| Move a task or report what an agent is doing | `mv`, `state` |
| Join this session to an agent, leave it, or list the agents | `connect`, `disconnect`, `agent ls` |
| Record a decision that matters later | `history add` |
| Switch between sparring, sleep, and brainstorm mode | `mode` |
| Record a decision a subagent made in sleep mode | `decision add` |
| Change a task's text | `edit` |
| Plan tasks for a release | `version`, `edit --planned-version`, `epic plan` |
| Attach images | `attachment` |
| Deliver a result to the requester | `result add`, `result ls` |
| Group and order tasks | `epic`, `progress` |
| Wait for a change | `wait` |
| Manage projects and folders | `project`, `workspace` |
| Open the board in the browser | `open` |
| Get the phone address and passcode, or see the remote access settings | `show remote` |
| Save the remote access settings (ngrok or Tailscale, domain, token) | `setup remote` |
| Run the server and the hook | `serve`, `stop`, `hook` |
| Install and update the Agent Project plugin | `setup`, `update` |
| Delete things (people only) | `rm`, `project rm`, `epic rm`, `init` |

## Connect a session to an agent {#connect-a-session-to-an-agent}

A person registers agents (a name and a one-line role) and issues a one-time connection code (valid for ten minutes) with the board's web API on the local entrance; the settings screen for this is not part of this revision. The session exchanges the code:

```bash
agpj --project "My project" connect K7M2QX9P
agpj --project "My project" agent ls
```

`connect` is refused while `AGPJ_CONNECTION_SECRET` is set (a session the board launched already has its connection). Otherwise it keeps the connection's secret in a file only you can read, tied to this session; the board keeps only its digest. Every new AI task belongs to the default agent, regardless of its epic; a person's task has no agent. Todo plans remain co-editable, subject to the existing draft and archive rules. While the default agent is the only live agent, sessions act as it without connecting. After another live agent is registered, only the assigned agent's connected session can start an AI task or change it from Doing onward. A missing connection returns `not_connected`, a replaced connection `connection_replaced`, and a different agent `not_assignee`. A session cannot use `--actor human` to bypass these checks. A person can reassign a task; a former assignee's later writes are refused. A second session cannot take an agent over (`agent_busy`); the person issues a new code with `switch` and gives it to you.

A session holds one agent: `connect` is refused while the session already holds an active connection. To leave the agent on purpose, run `agpj --project "My project" disconnect`. Its running tasks go back to waiting, the connection is revoked, the stored secret is deleted, and the agent is free again (a plain code connects it, and with nothing in Doing it can be deleted).

When the session ends, the Claude Code `SessionEnd` hook puts its running tasks back to waiting (paused). A session that is killed sends no notice, so a running task whose connection shows no activity for 30 minutes is put back to waiting by the board.

## Add and inspect tasks

Add a task and inspect the board:

```sh
agpj --project "My project" add "Write the setup guide" --assignee ai --actor ai
agpj --project "My project" ls --json
agpj --project "My project" show 1 --json
agpj --project "My project" snapshot --json
```

`--assignee` names the owner. `--actor` names who makes the change and controls permissions: use `human` for a person and `ai` for an agent. `snapshot` returns the complete board and history, including epics and any file problems.

## Report work

Report the start of work:

```sh
agpj --project "My project" mv 1 doing --actor ai --reason "Start implementation"
agpj --project "My project" state 1 running --actor ai --reason "Writing and checking the guide"
```

Moves and state changes accept an optional `--reason` of up to 10,000 Unicode characters. A missing or blank reason is generated from the current state. These operations add history by themselves; do not add a second entry for the same change. A question can be up to 5,000 characters, and an answer note up to 2,000 characters.

Record a decision that does not change the column or state but matters later, such as keeping a card in Todo:

```sh
agpj --project "My project" history add 1 --reason "The public wording depends on the result of checking on a real device, so the card stays in Todo until that check is done."
agpj --project "My project" history add 1 --reason-file progress.txt
```

The command assigns the timestamp and applies AI write permissions; it needs no actor, event ID, kind, or time. Write the grounds, the decision, and when known the condition for reconsidering. Reading the list needs no entry, and `result add` records itself. A missing or blank reason produces a default that matches the state. An AI cannot append history to a person's drafts, archived tasks, or Done tasks.

An option that takes text takes the next word as its value only if that word does not start with `-`. A missing value, as in `--reason --json`, is an `invalid_argument` error instead of recording the next flag as the text. To pass text that starts with `-`, such as a Markdown list item, join it to the option with `=` (`--reason="- Checked the links"`) or give it in a file (`--reason-file`, `--text-file`, `--desc-file`, `--document-file`) or on standard input (`--desc -`, `--questions-json -`). A lone `-` is still read as standard input where the option allows it. A whole `TASK.md` or `EPIC.md` starts with `---`, so give it with `--desc-file` or `--document-file`, or on standard input. The options affected are `--reason`, `--text`, `--desc`, `--document`, `--questions-json`, `--title`, `--summary`, and `--link`.

### Deliver a result {#deliver-a-result}

Record the first result the requester can look at, even while the work goes on, and keep working. A result is short text (`--summary`) with an optional heading (`--title`, 1 to 80 characters on one line), optionally with images (`--file`, repeated for several images, up to 20 in the order given) and a link or storage location of the main artifact (`--link`). Without `--final` the result is in progress; `--final` marks the finished one.

If an AI task has all its conditions met but no final result, `agpj progress` and the agent hook list it under `finalResultMissing` so the agent records one. This never blocks or delays automatic Done, and a Done task still accepts its first final result.

```sh
agpj --project "My project" result add 1 --title "First draft" --summary "The first 30 seconds are ready" --link ./out/draft.mp4 --actor ai
agpj --project "My project" result add 1 --summary "Top page capture" --file top.png --actor ai
agpj --project "My project" result add 1 --summary "All screens of the mock" --file 01.png --file 02.png --file 03.png --actor ai
agpj --project "My project" result add 1 --summary "Finished and checked" --final --actor ai
agpj --project "My project" result add 1 --published v1.0.0 --link https://example.com/releases/v1.0.0 --summary "Released" --actor ai
agpj --project "My project" result add 1 --plan RP-2 --summary "The endpoint answers its first request" --link https://example.com/demo --actor ai
agpj --project "My project" result add 1 --plan RP-3 --actor ai
agpj --project "My project" result ls 1
```

`--plan RP-<n>` delivers a result against one of the task's [result plans](../../dev/reference/files.md#result-plans), in any order and without retyping the plan: the title defaults to the plan's content (`--title` overrides it), `--summary` may be left out and then reads as the plan's content, and the result carries the plan's ID as `planId`. A plan counts as delivered while a saved result carries its ID, and `show --json` returns that as `deliveredResultVersion` on each plan (`null` while undelivered). A final plan makes the result final without `--final`; `--final` with an intermediate plan fails with `invalid_value`, and `--final` without `--plan` works as before. An unknown plan ID, or one of another task, fails with `result_plan_not_found`. A plan is delivered by one result: delivering it again with different content fails with `result_plan_delivered`, while repeating the same delivery (recognized as below, with the plan part of it) is safe. `--plan` cannot be combined with `--published`. Intermediate plans that are not delivered never block work or automatic Done, and the `finalResultMissing` guidance is unchanged.

When an agent starts or resumes a task, the progress table (`agpj progress`, and what the hook gives the agent) lists the plans of that task that are not delivered yet, as `resultPlans` with each plan's ID, content, and delivery condition (the text output prints an `undelivered RP-n: … - deliver when: …` line under the task). The agent needs no extra command for this. When it reaches a condition it delivers that plan with `result add <id> --plan RP-<n>`, and the next table lists only the plans that remain; it keeps working without waiting for a reply. A task with no plans, or with every plan delivered, shows no `resultPlans`, and results work as before.

Each result gets the next version number of the task (1, 2, and so on). Recording one changes neither the column nor the run state, and adds one history entry. Repeating the same delivery is safe: the command finds the result it already recorded, reports `created: false`, and writes nothing. Without `--key` the delivery is recognized by its content (kind, summary, link, and the image's bytes); give `--key <key>` to name it yourself. Reusing a key for different content fails with `invalid_value`. Without `--title`, the title is the first sentence of the summary, cut to 80 characters (the last one is `…` when it was cut); the title is part of what makes a delivery the same one, so repeating a delivery with another title is a different result unless you pass the same `--key` (which then fails with `invalid_value`). The summary and the link are 1 to 2,000 characters, and a task keeps up to 100 results. An AI cannot record a result on a person's draft, an archived task, or a Done task, except that repeating a result that was already recorded succeeds. `--published VERSION` records a publication: it needs `--link`, cannot be combined with `--final`, validates the version name like `version add`, and starts the summary with a `Published: VERSION` line. It is the one result a Done task takes, even when the task already has a final result, from an AI or a person.
### Finish a task

To finish, save verification in the task body with `edit`, then run `agpj --project "My project" acceptance check 1 AC-1` for each verified condition. Do not edit `TASK.md` directly from Doing onward. The server moves eligible AI tasks to Done automatically (the hook and waiting commands start it when needed), so do not run `mv ... done --actor ai`. See the [Markdown file format](./file-format) for the rules. A Done task is read-only for agents; a user can reopen it when more work is needed.

### Ask a question

Use `message ask` to ask the user up to 20 questions. Each question is its own message, such as `Q-12`, and belongs to the project; `--task <id>` makes a task refer to them, and the task shows Awaiting answer until they are answered. If the task is an AI task in Doing and Running, `--task` (and `message link`) pauses it in the same save and adds the history entry "Waiting for the answers to Q-1." ("Q-1 の回答を待ちます。" in a Japanese project). When the agent takes in the answers (`message accept`, or the answered result of `message ask` or `message wait`), the task goes back to Running and the history records "Resumed after the answers to Q-1." This happens only when every question the task refers to is answered and the task is an AI task in Doing and Paused; accepting one answer does not resume it while another question it refers to is still open. The command prints one event per line (JSON Lines with `--json`) and waits until none is open, 600 seconds by default:

```sh
agpj --project "My project" message ask --actor ai --task 1 --questions-json - --json < questions.json
```

It ends with `answered` (exit 0), `timeout` (exit 5, the questions stay open), or `withdrawn` (exit 6). Wait again with `message wait Q-12 Q-13`. Waiting needs the server: the command starts it in the background when none is running, before it saves the questions. `--no-wait` registers the questions and returns at once without one. `message answer`, `message ls`, `message show`, `message withdraw`, `message link`, and `message unlink` record an answer, read messages, withdraw a question, and change which tasks refer to a question. `message send` and `message accept` handle requests from a person to the agents. A question can carry pictures with `images`, a list of image file paths that the command saves with the question (see [Decisions](../delegate/decisions)). See [Decisions](../delegate/decisions) for the question format and every command. `progress` also lists up to three recommended tasks as `nextTasks` (see [Epics and progress](../watch/epics#read-the-progress-table)). For epics and the progress table, see [Epics and progress](../watch/epics).

## Sleep mode {#sleep-mode}

`mode` shows or switches the project's mode. Sparring is the default: the agent listens and asks you on the board. In sleep mode questions are held until sleep ends.

```sh
agpj --project "My project" mode            # show the current mode
agpj --project "My project" mode sleep      # stop questions to you
agpj --project "My project" mode sparring   # back to the default
```

A daily window puts the project to sleep by itself. Times are `HH:MM` on this computer's clock; an end earlier than the start crosses midnight.

```sh
agpj --project "My project" mode schedule --start 23:00 --end 08:00   # set and turn on the window
agpj --project "My project" mode schedule --off                        # turn it off (the times are kept)
agpj --project "My project" mode auto                                  # forget the manual mode; the window decides
```

A manual mode wins while it is set. One set during a window holds until that window ends, and then the schedule takes over again; one set outside a window holds until the next window starts (with no window, until you change it). `mode` shows the mode in effect, where it comes from (`manual` or `schedule`, with `until` when the mode ends: the window's end, or for a manual mode the end of its hold), and the window. `message ask` and `agpj progress` use the same resolution; the progress table adds `modeSource` and `modeUntil` next to `mode`.


### Brainstorm mode {#brainstorm-mode}

Brainstorm mode asks the agent to question you about one epic or one task. Start it with the target, a task number or an epic ID, and end it by returning to sparring:

```sh
agpj --project "My project" mode brainstorm --target INTERACTION-MODES   # an epic
agpj --project "My project" mode brainstorm --target 12                  # a task
agpj --project "My project" mode sparring                                # end it
```

`--target` is required, and it is accepted only with `brainstorm`. A target that does not exist is refused, and so is an archived task. Starting a brainstorm also sends the agent a request, "Start brainstorming on ...", so an agent that is listening with `wait` begins at once. The progress table then shows `mode: "brainstorm"` and `modeTarget`, and `mode` prints the target.

The agent asks its questions with `--brainstorm`. They are not linked to a task, so the target task keeps running:

```sh
agpj --project "My project" message ask --actor ai --brainstorm --questions-json '{"questions":[{"body":"Who reads this page first?"}]}'
```

`message ask --brainstorm` is refused with `brainstorm_not_active` once the mode is no longer brainstorm. Questions without `--brainstorm` are ordinary and are still accepted. Sleep always wins: when a sleep window starts, the mode becomes sleep and every `message ask` is held, with or without `--brainstorm`.

In sleep mode `message ask` is not refused: it stores the questions as held and prints `Held 2 questions: Q-5, Q-6.` (`{"type": "held", "questions": [{"id": "Q-5"}, {"id": "Q-6"}]}` with `--json`) at once, even without `--no-wait`. A held question does not reach the board, the notifications, or the chat, and its tasks keep working. `agpj message withdraw <id> --actor ai` withdraws one, and `agpj message wait` on a held question returns at once. When sleep ends, by `agpj mode sparring` or the end of the daily window, `agpj mode` prints `Released 2 held questions: Q-5, Q-6.` and they behave like ordinary open questions. The agent has a subagent decide what it can, continues with the conclusion, and records it on the task:

```sh
agpj --project "My project" decision add 1 \
  --question "Delete or keep the old rows?" --choice "Keep and migrate" \
  --reason "Deleting cannot be undone." --decided-by subagent:reviewer \
  --basis "src/file_store/ops/task.rs" --basis "Q-12"
```

`--decided-by` must look like `subagent:NAME`, and `--basis` is required: give each code path, document, or past question (`Q-<n>`) the decision rests on. The command adds one history entry, `Decided in sleep mode by subagent:reviewer. Question: ... Choice: ... Reason: ... Basis: ...`, so the card's history shows what was decided, by whom, why, and on what grounds. The mode is saved per project and appears as `mode` in `agpj progress --json`.


## Edit long text from a file

For longer Markdown, use a file rather than a command-line string:

```sh
agpj --project "My project" edit 1 --desc-file task.md
```

Descriptions allow up to 50,000 characters and epic details (`EPIC.md`) up to 100,000, and the file must follow the [Markdown file format](./file-format). Use `--desc -` to read a description from standard input.

`edit --desc-file` replaces the body, the acceptance conditions, and the result plans (`resultPlans`: what the task will deliver and the condition for delivering it; see [Result plans](../../dev/reference/files.md#result-plans)), and keeps the latest history and the question references (`questions`) from `TASK.md`; `history` and `questions` in the input file are ignored, so a task that refers to an open question stays Awaiting answer. Use `message link` and `message unlink` to change the references. Moves, state changes, and other operations from the CLI or the UI append history to `TASK.md`, and `show`, `snapshot`, and the browser read that same history.

## Edit epic details

An epic's details are the whole `EPIC.md` text, frontmatter included. Pass the text directly, or read it from a file or standard input:

```sh
agpj --project "My project" epic edit LAUNCH --document-file epic.md
agpj --project "My project" epic edit LAUNCH --document -
agpj --project "My project" epic edit LAUNCH --waves 3 1,2
```

`--document` and `--document-file` cannot be given together, and each accepts up to 100,000 characters.

`--waves` sets the order of the epic's tasks. Give one value per wave, either a task ID or task IDs joined by commas. Tasks in one wave can run in parallel, and a wave starts when every task of the waves before it is Done. The example above puts task 3 in the first wave and tasks 1 and 2 in the second. See [Order the work](../watch/epics#order-the-work).

An agent's `add` creates an ordinary Todo task. For a request for the plan only, it adds `--backlog` to put the card in the Backlog instead. `epic add` has no lane: an agent's epic is an ordinary epic, and a plan-only epic is one whose cards are created with `--backlog --epic <ID>`:

```sh
agpj --project "My project" add --title "Plan the launch" --assignee ai --actor ai --backlog --epic LAUNCH
```

The Backlog holds settled plans that must not be started yet. An agent never starts them, and its `mv` and `state` on a Backlog card fail with `task_in_backlog` (exit code 1). A person sends a card to Todo, at the end of the lane; an agent does so only when the user told it to, and says so in `--reason`. From the command line, send one card by moving it to `todo`; there is no separate `send` command:

```sh
agpj --project "My project" mv 12 todo --actor human
agpj --project "My project" epic send LAUNCH --actor human
agpj --project "My project" epic send LAUNCH --actor human --tasks 1,2
```

`mv <id> todo` sends one card; moving a card that is in the Backlog to Doing or Done is a person's move; an agent gets `task_in_backlog`. `epic send` sends the whole epic, or only the cards given with `--tasks`, in one save that either moves all of them or none. An agent may run either one only when the user told it to; it cannot confirm a send with `--force` (`human_only`). An agent that moves a task from Doing back to Todo (`mv <id> todo` on a Doing task) gets `human_only` too; only a person does that. An agent may pause its own Running task with `state <id> paused --actor ai --reason "<why>"`, which it does only when the stalled-task reminder flags the task and it is not working on it. An agent does not move a task to Done; eligible tasks move there automatically (see Finish a task). A card that is already out of the Backlog is moved like any other card, and a card a person is editing in the browser (`draft: human`) cannot be sent (`task_is_draft`). The history records "A person sent the task to Todo." or "An agent sent the task to Todo." followed by the `--reason` the agent gave.

If the plan has open questions, the send is refused with `send_with_open_questions` (exit code 1) and the message lists the question IDs. Add `--force` to `mv` (or `epic send`) to send anyway; the questions stay open and the card is Awaiting answer until they are answered:

```sh
agpj --project "My project" mv 12 todo --actor human --force
```

A person can take an unstarted Todo card (Idle, no results) back to the Backlog; an agent gets `human_only`:

```sh
agpj --project "My project" mv 12 backlog --actor human
```

While a person has an epic in draft in the browser, `epic edit` fails with `epic_is_draft` (exit code 1) and changes nothing, `--waves` included. Wait until the person saves or discards it, or ask them to finalize a draft that was left behind. `epic attachment` and changing a task's epic still work.

## Filter and list

```sh
agpj --project "My project" ls --column doing --json
agpj --project "My project" ls --column backlog --json
agpj --project "My project" ls --awaiting-answer --json
agpj ls --all-projects --json
```

`--state` takes `idle`, `running`, or `paused`; `--awaiting-answer` lists only the tasks that wait for an answer to an open question. `ls --all-projects` reads every project in name order, with complete task descriptions and history. Its `--column` and `--state` filters apply to every project; do not combine it with `--project`.

## Plan tasks for a version {#plan-tasks-for-a-version}

A project lists its versions and can name one of them as the release version. A task is planned for one version, or for none (unplanned). A version name is 1 to 64 characters with no whitespace, such as `v1.8.0`; it is kept and compared exactly.

```sh
agpj --project "My project" version add v1.9.0
agpj --project "My project" version release v1.8.0
agpj --project "My project" version release
agpj --project "My project" version release --clear
agpj --project "My project" version ls --json
```

A project's versions are the `*.md` files directly inside `docs/release/` of its [Docs](../watch/docs), listed newest first (`v1.10.0` before `v1.9.0`; `TEMPLATE.md` is the release template and is never a version). `version add VERSION` creates `docs/release/VERSION.md` as a copy of `docs/release/TEMPLATE.md` (a standard release-note template is written there first when it is missing) and never overwrites an existing file, so a version that already has its file is already listed. It also puts one release-work card in the Backlog, owned by the AI, titled `Release VERSION` (`VERSION をリリースする` when the board language is Japanese) and planned for VERSION. Its to-do lists writing the release notes in `docs/release/VERSION.md` from the template and doing the release work itself (publishing the release and confirming that it reached users): the work starts only after a person sends it to Todo, and adding the same version again issues no second card. When that card reaches Done, by any path, and VERSION is the release version, the release version moves to the nearest newer listed version and the card's history says so (`Release version moved from v2.1.0 to v2.2.0.`). When no newer version is listed, the release version stays, and the history says so. Nothing changes when the release version is another version, when the card moves to a lane other than Done, or when the card is already Done. Only the first time the card reaches Done can move the release version: if you reopen the card and finish it again, the release version stays, even when a newer version has been listed since. Adding a version does not change the release version. You can also add a version by creating the file yourself. `version release VERSION` lists the version the same way when it is not listed yet and makes it current. Without an argument it shows the release version. If the release version's file is deleted later, the project reads as having no release version until the file returns. Changing the release version moves no task: a task stores the version name it was given.

### Generate a release document {#generate-a-release-document}

```sh
agpj --project "My project" release doc --release-version
agpj --project "My project" release doc --version v1.9.0
```

`release doc` writes the release document of one version to `docs/release/VERSION.md` in the project's [Docs](../watch/docs). It reads every card of the project, archived ones included, and groups them for that version:

- **Planned**: the cards whose planned version is this version now.
- **Verified**: planned cards that have acceptance conditions, all of them done.
- **Done without acceptance conditions**: planned cards in Done that have no acceptance conditions at all. Nothing can verify them, so they are neither verified nor remaining.
- **Remaining**: the other planned cards that are not verified yet.
- **Moved to another version**: cards that left this version, whether to a later version, an earlier one, or none. Their history holds a change of planned version away from it, and each line names where the card went.

Each item names its card (`#12`), the number of results and history entries, and each result with its time and link, and points to `agpj show 12` for the original records. The last section, **Publication**, separates published cards from the rest. A planned card counts as published only when one of its results has a line `Published: VERSION` in its summary and a link to what was published; a card whose result has the line but no link, and a card with no such result, are listed as not confirmed, however finished they are. The summary also shows `Published (with a publication record): N / M`. See [Version reports and theme retrospectives](../delegate/release-reports) for how agents use this.

The document is safe to edit by hand. The generated part sits between the lines `<!-- agpj:generated:start -->` and `<!-- agpj:generated:end -->`. Generating again replaces only what is between them, so text above or below stays, and generating twice from the same board changes nothing. A file without those lines, such as the one `version add` creates from the template, keeps its text and gets the block added at the end. A file with a missing, repeated, or reversed marker line is refused with `release_doc_refused` and left as it was, as is a file that changes while it is generated. The file is saved in one step, so a failure never leaves half a document.

`--version VERSION` generates for that version and lists it first when it is not listed, as `version add` does. `--release-version` uses the project's release version; with none set it fails with `no_release_version` (exit code 1) and writes nothing. Each version has its own file, so generating for another version adds a second document and leaves the first alone. Open the file from Docs in the board, where `docs/release/` shows the versions.

Generating never publishes anything. It writes one file inside the project's shared folder, and sends nothing to GitHub or any other service.

A card's planned version is the `plannedVersion` key in the frontmatter of its `TASK.md`, so you can also set or remove it by hand.

Plan a card when you add it, or later:

```sh
agpj --project "My project" add "Fix the login bug" --assignee ai --actor ai --release-version
agpj --project "My project" edit 15 --planned-version v1.9.0
agpj --project "My project" edit 15 --release-version
agpj --project "My project" edit 15 --unplanned
```

`--planned-version VERSION` needs a listed version (`version add`, or a file in `docs/release/`), `--release-version` stores the release version's name as it is now, and `--unplanned` removes the plan. The three exclude each other. When the project has no release version, `--release-version` fails with `no_release_version` (exit code 1) and changes nothing. Each real change is recorded in the card's history; planning a card for the version it already has changes nothing. The planned version is the one field you can change on a Done or archived card, as an agent or a person; every other edit of a Done card stays refused for an agent, so an edit that combines `--planned-version` with a title or description change on a Done card is refused as a whole. An agent cannot change a person's draft.

Plan every card of an epic at once with `epic plan`:

```sh
agpj --project "My project" epic plan LAUNCH --planned-version v1.9.0
agpj --project "My project" epic plan LAUNCH --release-version --only-unplanned
agpj --project "My project" epic plan LAUNCH --unplanned
```

Each card is saved on its own, so a failure part way leaves the earlier cards changed and running the command again finishes the rest. The result counts the cards that were `changed`, `unchanged` (already had the version), and `skipped`, and names them. Archived cards, Done cards (they shipped), and cards a person is drafting are always skipped. With `--only-unplanned`, cards that already have a planned version are skipped too; `--unplanned` and `--only-unplanned` cannot be combined.

Narrow `ls` and `progress` to one version:

```sh
agpj --project "My project" ls --release-version
agpj --project "My project" ls --version v1.8.0 --epic LAUNCH --json
agpj --project "My project" ls --unplanned --column todo
agpj --project "My project" progress --version v1.8.0 --json
```

`--version VERSION`, `--release-version`, and `--unplanned` exclude each other, and combine with the other filters. `--release-version` with no release version fails with `no_release_version`; `ls --all-projects` takes `--version` and `--unplanned` but not `--release-version`, because each project has its own. Every task in the JSON carries its `plannedVersion` (left out when the task has none), and the text shows `[plan VERSION]`. The root `agpj --version` still prints the version of the program.

A version filter only hides tasks. In `progress`, which tasks are ready for an agent is decided on the whole board first, so a task whose earlier-wave tasks are hidden by the filter is still not ready, and `nextTasks` holds the first three ready tasks that the filter keeps. Requests, answers, and epics to close are not narrowed.

## Projects

```sh
agpj project create "My project"
agpj project ls --json
agpj --project "My project" project rename "Website launch"
agpj project bind "My project" --folder ./docs
```

Renaming keeps the project's ID, folder registrations, and tasks. Folder registrations and project deletion are described in [Data and backups](../operations/data#projects-and-folders).

## Wait for changes or a task

The `wait` command has two modes. `--after <cursor>` waits for a project change as described in [Working with agents](../delegate/agents#wait-for-changes). `--until task` returns one eligible AI task, including one that was already in Todo or an idle task in Doing before listening started:

```sh
agpj --project "My project" wait --until task --idle-timeout 300 --json
```

`--idle-timeout` is required with `--until task` and accepts 1 to 86,400 seconds. It has no default. Each invocation returns at most one task, in the same order as the progress table; it does not claim or change the task. After resolving the returned task, call `wait` again to continue. See [Working with agents](../delegate/agents#listen-for-the-next-task) for the full session workflow and how Awaiting answer tasks are handled.

On success, task mode prints one JSON object and exits 0:

```json
{"kind":"task","projectId":"<id>","cursor":"<cursor>","task":{"id":1},"taskPath":"…/tasks/1/TASK.md","documentPath":null}
```

Before a task, the wait can return `{"kind":"answers",…}` (answers to questions that no agent has taken in) or `{"kind":"requests",…}` (open requests), answers first. Take each in with `message accept` and wait again.

When there has been no eligible task for the full interval, it prints `{"kind":"idle_timeout","projectId":"<id>","idleTimeoutSeconds":300}` and exits 0. This is a normal idle completion; it does not say that previous work succeeded. `board_unavailable`, `project_not_found`, invalid board data, and invalid arguments remain errors with non-zero exit codes. `--timeout` applies only to `--after` mode; it reports `wait_timeout` as an error and does not answer a question.

## Run the server

You rarely need to start the server yourself. `wait`, a question that waits for its answer, and the agent hook start it in the background when none is running. A command that starts it prints one line on standard error:

```text
Started the Agent Project server at http://127.0.0.1:7420/ in the background. It stops by itself after 30 minutes without use.
```

A server started this way keeps running while the board is open in a browser, while an agent waits, or while the phone is connected. It stops after 30 minutes without any of these; set `AGPJ_IDLE_TIMEOUT` (60 to 86,400 seconds) to change that. If port 7420 is taken, it uses a free port and says so. Set `AGPJ_AUTOSTART=0` to turn automatic starts off; commands that need the server then fail with `board_unavailable`, as when no server runs. Other commands, such as `add`, `mv`, and `ls`, never start the server.

### Open the board in the browser

A server that starts in the background opens the board of the project it started for, once, in your default browser. Before it opens the page, it waits about three seconds: if a board tab that was already open connects again, for example after the computer woke from sleep, it opens nothing. A question that arrives while the server runs does not open the browser, and neither does `agpj serve` or `agpj serve -d`.

The browser is not opened when:

- `AGPJ_OPEN_BROWSER=0` is set;
- the command runs in an SSH session (`SSH_CONNECTION` or `SSH_TTY` is set);
- on Linux, no display is available (`DISPLAY` and `WAYLAND_DISPLAY` are unset).

Whether it opened the page, and why not, is written to `serve.log` in the data folder.

To open the board yourself, run `agpj open`. It opens the board of the project bound to the current folder, or of the one `--project` names, and starts the server in the background first when none runs:

```sh
agpj open
agpj --project "My project" open
agpj open --print
```

It prints the board's address, such as `http://127.0.0.1:7420/?project=<id>`, even when it opens it. `--print` only prints the address. With `--json`, it prints `{"url":"…","projectId":"…","opened":true}`; when the browser was not opened, `opened` is `false` and `notice` says why. With `AGPJ_AUTOSTART=0` and no server running, it fails with `board_unavailable`; a program that cannot be started fails with `browser_open_failed`, and the message names the address to open yourself.

Agent Project uses `open` on macOS and `xdg-open` on Linux. To use another program, set `AGPJ_BROWSER_OPENER` to it; Agent Project runs it with the address as its only argument, also over SSH or without a display.

### Start the server yourself

Start it yourself to keep it running, to choose the port, or to connect a phone when it starts.

`agpj serve` starts the board on `127.0.0.1`. `--port` chooses the port (default 7420). To also connect a phone through ngrok when the server starts, add `--monitor ngrok`:

| Option | Meaning |
| --- | --- |
| `--monitor ngrok` | Starts the remote access connection. `ngrok` is the only mode. Needs `NGROK_AUTHTOKEN` in the environment; the token is never passed as an argument. To keep the same address at every start, enter a domain your ngrok account owns in the dialog, or set `AGPJ_NGROK_DOMAIN`, or save it from the command line with [`agpj setup remote`](#save-the-remote-access-settings); see [Keep the same address](../watch/remote-access#keep-the-same-address). |

The phone is guarded by a 4-digit passcode that is made for each start; there is no sign-in option. An invalid `--monitor` value, a missing `NGROK_AUTHTOKEN`, or an unknown option exits with code 2. If the connection cannot start, `serve` exits with code 1 and does not start.

When the connection is ready, the startup line shows the phone address and the passcode: `Serving at http://127.0.0.1:7420/; remote access at https://… with passcode 4821`. With `--json`, the startup line has the `url` of the board and `monitor` with `mode: "ngrok"`, and the phone `url` and `passcode` in `ngrok`. The passcode is printed only here and in the board's connection dialog; tokens and policies are never printed. You can also set up the connection from the board's **Settings**; see the [remote access guide](../watch/remote-access).

### Run in the background

`agpj serve -d` (or `--daemon`) starts the server in the background and returns once it is ready, so the terminal is free and closing it does not stop the server. It prints the same address (and, with `--monitor ngrok`, the phone address and passcode) plus the process ID and the log file. `--port` and `--monitor` work as usual; with `--json`, the startup object also has `pid` and `log`.

```sh
agpj serve -d
agpj stop
```

`agpj stop` shuts down the running server and returns when it has exited. It stops a server started with `serve`, with `serve -d`, or by a command. After `stop`, the hook does not start the server again until a command that waits, `serve`, or `serve -d` starts one. If no server is running, it says so and exits with code 0. `stop` takes no `--project`.

### Give the phone address and passcode

While the ngrok connection is up, `agpj show remote` with `--project` prints its state, the public address, and the 4-digit passcode, so that an agent can hand you a link without you opening the connection dialog:

```sh
agpj --project "My project" show remote
agpj --project "My project" show remote --json
```

```text
ngrok: ready
URL: https://….ngrok-free.app/?project=<id>
Passcode: 4821
Project: My project
Enabled: yes
Domain: example.ngrok-free.dev
Token: saved in the keychain
```

The address opens that project. With `--json` it prints `{"status":"ready","url":"…","entrance":"…","passcode":"…","project":{"id":"…","name":"…"},"enabled":true,"domain":"…","tokenSaved":true,"keychainAvailable":true}`; `entrance` is the address without a project. The last four fields are the saved settings below.

The plugin ships the skill `remote` (`/agent-project:remote` in Claude Code) for this: ask your agent for the remote access URL or the phone link, and it runs `agpj --project "<name>" show remote --json` for the project of the folder and replies with the address and passcode. It never starts the server or ngrok.

With `--project`, `show remote` starts nothing. When no server is running, or the connection is not ready, it prints no address and no passcode, says why, and exits with code 1 as `remote_unavailable`. A project that does not exist exits with code 4.

Without `--project`, `show remote` shows the saved remote access settings and the state of the connection, and never fails because the connection is down:

```sh
agpj show remote
agpj show remote --json
```

```text
Enabled: yes
Domain: example.ngrok-free.dev
Token: saved in the keychain
Connection: ready at https://….ngrok-free.dev
```

It shows whether remote access is turned on, the saved fixed domain (or that ngrok chooses an address at each start), whether a token is saved in the keychain, and whether the keychain is available. The token itself is never shown, and no passcode is printed without `--project`. With `--json` it prints `enabled`, `domain`, `tokenSaved`, `keychainAvailable`, `status` (`no_server` when no server is running, otherwise the state of the connection such as `ready`, `disabled`, or `failed`), and `entrance` (the address while the connection is ready, else `null`). `show <id>` still shows a task; `remote` is the only word that is not a task ID.

### Save the remote access settings {#save-the-remote-access-settings}

`agpj setup remote` saves the remote access settings that the board's connection dialog also keeps: whether the connection is on, whether the phone connects through ngrok or Tailscale, and for ngrok the fixed domain and the token. It needs no project, because the settings belong to the whole data folder, and it works whether or not a server is running. A running board shows the change without a reload.

```sh
agpj setup remote
agpj setup remote --provider tailscale --enabled true
agpj setup remote --domain example.ngrok-free.dev --enabled true
agpj setup remote --clear-domain
printf '%s' "$NGROK_AUTHTOKEN" | agpj setup remote --token-stdin
```

On a terminal, `agpj setup remote` with no option shows what is saved (the provider, the domain, and whether a token is saved, never the token) and asks which way the phone connects, ngrok or Tailscale. It then asks only the settings of that way: for ngrok the domain and the token, and for Tailscale nothing more, because Tailscale needs neither. The token is typed hidden and is never printed. Each question can be skipped by leaving it empty, and a value that is already saved is changed only when you say so. Without a terminal, or with `--json`, it asks nothing and exits with `2`, so a script never waits for input.

When it saves Tailscale (by choosing it or with `--provider tailscale`), it also checks Tailscale on this computer and says what to do next: install it (the Mac app, or the Homebrew command-line tool, which cannot be used together), sign in, turn on MagicDNS and HTTPS certificates, or, when everything is ready, the `ts.net` address the phone will open. The check is advice only. It never blocks the save, and the command still exits with `0`. With `--json` the result is a `tailscale` object with a `status` (`ready`, `not_installed`, `not_logged_in`, or `https_disabled`) and the `domain` when ready.

The options save without any question:

| Option | Meaning |
| --- | --- |
| `--provider ngrok\|tailscale` | Saves how the phone connects. `ngrok` is the default. With `tailscale` the domain and the token are not used, and `setup remote` shows neither. Any other name exits with code 2. See [Choose ngrok or Tailscale](../watch/remote-access#choose-a-way). |
| `--domain <host>` | Saves a fixed ngrok domain your account owns. It must be a bare host name; it is lower-cased, and a value with `https://`, a path, or a port exits with code 2 as `invalid_domain`. |
| `--clear-domain` | Removes the saved domain, so ngrok chooses an address at each start. Cannot be combined with `--domain`. |
| `--enabled true\|false` | Saves whether the connection is turned on. |
| `--token-stdin` | Reads the ngrok token from standard input and saves it in the operating system's keychain, the same place the board's **Save the token on this PC** uses. The token is never accepted as an argument. Without a usable keychain nothing is saved and the command exits with code 1 as `keychain_unavailable`. |

Saving does not start, stop, or replace a connection. A connection that is already up keeps running as it is, and the saved settings apply the next time the connection starts; the command says so. The settings are saved in the same file as the board's own, so the board and the command always show the same values. `setup remote` accepts `--json` and takes no `--project`; it cannot be combined with the options of `setup`.

The server gives the passcode only to the local address of your PC, so it is printed to the person running the command and never travels through ngrok. If an agent pastes it into a chat, everyone who can read that chat can open the board for as long as the connection is up. Stop the connection when you are done; see the [remote access guide](../watch/remote-access).

The background server writes its output to `serve.log` in the data folder. The file is readable only by you, because it holds the phone passcode, and it is replaced at every `serve -d`. If the background server cannot start (for example the port is in use), `serve -d` exits with code 1 and shows the reason. The server does not restart by itself after a reboot.

If a command started the running server, `serve -d` keeps that server running instead of starting a new one: it no longer stops by itself, and nothing it serves is interrupted. This needs no `--port` or `--monitor` that differs from the running server. A foreground `agpj serve` exits with `already_serving` and points to `serve -d`.

If a server you started is already running, or the options differ, `serve -d` asks in the terminal whether to stop it and start a new one. Answer `y` to replace it; the new server uses the `--port` and `--monitor` of this command, not those of the old server, and with `--monitor ngrok` it gets a new passcode. Any other answer leaves the running server as it is and exits with code 0. With `--json`, or when the command does not run in a terminal, nothing is asked and `serve -d` exits with code 1 (`already_serving`).

## Setup and update {#setup-and-update}

`setup` creates the data folder and installs the Agent Project plugin (the Skill and the Hook) for Codex, Claude Code, or both. `update` moves an installed plugin to the version of the command. What they ask and change is in [Working with agents](../delegate/agents#set-up-your-agents).

```sh
agpj setup
agpj setup --agent codex --dry-run
agpj setup --agent all --yes --json
agpj update
agpj update --yes
```

| Command | What it does | Exit codes |
| --- | --- | --- |
| `agpj setup` | Creates the data folder, asks which agents to set up, shows the plan, and after you answer `y` installs the plugin, lets the agents use the shared folder, and trusts the Agent Project hooks in Codex. Needs no project. | `0` success, nothing to change, or a cancel. `1` an agent failed, or no agent could be set up. `2` bad command line, or missing options without a terminal. |
| `agpj setup remote` | Saves the remote access settings: the ngrok domain, whether it is on, and the token. Needs no project. See [Save the remote access settings](#save-the-remote-access-settings). | `0` success. `1` the keychain or the settings file cannot be used. `2` bad command line, an invalid domain, or no option without a terminal. |
| `agpj update` | Moves the plugin of every agent that has it to this version and trusts the new Codex hooks. Shows the plan and asks first. Needed after an upgrade only when the automatic update cannot do it (Codex hook trust) or to repair the plugin; see [Update the plugin](../delegate/agents#update-the-plugin). | `0` success, nothing to update, or a cancel. `1` an agent failed. `2` bad command line, or missing options without a terminal. |

`setup` installs the plugin again only when it is missing or its files differ from the ones your `agpj` ships; `update` always reinstalls it.

Both commands take these options:

- `--agent codex|claude|all` chooses the agents. Without it, `setup` asks on a terminal, and `update` takes every agent that has the plugin.
- `--yes` agrees in advance, including the trust of the Codex hooks.
- `--dry-run` shows the plan and installs, creates, and changes nothing.
- `--json` prints machine-readable output.

On a terminal, plain `agpj setup` also offers to set up remote access after the plugin is installed: it asks whether to do so (the default is no), and then asks the same questions as `agpj setup remote`. Skip it and nothing is asked again. It is not offered without a terminal, with `--json`, with `--yes`, or with `--dry-run`.

Neither command waits on a pipe. Without a terminal, or with `--json`, `setup` needs `--agent`, and both need `--yes` or `--dry-run`; otherwise they exit with `2`. An agent whose command is not on `PATH` is skipped with the reason. Failure reasons go to the error output. See [Data and backups](../operations/data#set-up-the-shared-folder) for the settings files they change and how they back them up.

## Skill text {#skill-text}

`agpj skills` prints the instructions of the `agent-project` skill, and `agpj skills <topic>` prints one reference or template: `project`, `language`, `files`, `requests`, `work-loop`, `questions`, `waiting`, `file-format`, `listening`, `resident`, `planning`, `fixed-epic-bodies`, `drafts`, `phone`, `task-template`, `epic-template`, `bug-epic-template`, or `enhancement-epic-template`. The plugin's `SKILL.md` runs `agpj skills`, so the agent always gets the instructions of the `agpj` you have installed. The command prints Markdown, writes nothing, starts no server, and always exits `0`; a name that is no topic prints the list of topics.

## Output and confirmation

Add `--json` for machine-readable output. The help text, errors, and JSON values are always in English, whatever language the browser uses.

Deleting a task, epic, or project, and emptying a project with `init`, needs a person's confirmation in a terminal. Agents must ask a person to do it.

## Next

- [Working with agents](../delegate/agents): where these commands fit in the work loop.
- [Troubleshooting](../operations/troubleshooting): what an error code means.
