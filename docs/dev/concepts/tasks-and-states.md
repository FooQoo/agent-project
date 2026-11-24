# Tasks and states

What a project and a task are, which lane and run-state combinations are valid, and how the Backlog, epics, drafts, and archiving fit in. For every field, see [Shared files](../reference/files.md).

## Projects

A project is a unit of work with a name and an immutable **ProjectId**, a 32-character lowercase hexadecimal string generated at creation. Each project owns exactly one board; there is no cross-project board and no moving or copying tasks between projects.

- A name is 1 to 50 Unicode scalar values after trimming, on one line, and unique by exact, case-sensitive match. Project lists are sorted by name.
- Rename and `init` keep the ProjectId. A deleted ProjectId is never reused, so a project re-created with the same name is a different project.
- Task IDs, lane positions, and EPIC-IDs are scoped to a project. The target project is never inferred from omission, the working folder, or the web selection.
- `nextTaskId` starts at 1 and only increases, even across deletion and `init`. A number reserved by a failed creation stays unused.
- `init` keeps the ID, name, and `nextTaskId`, and empties the project's tasks and epics. Deleting a project removes its whole folder and touches no other project.

Change detection uses three values. `version` is computed from one project's source files and has no order. `catalogVersion` does the same for the project list. A `cursor` (`f1:<ProjectId>:<version>`) is what `wait` uses; it follows the same ProjectId across renames and never transfers to another project.

## Lanes and run states

A task's lane says where the work is; its run state says what the agent is doing about it. Not every combination is valid:

| Assignee | Lane | Valid run states |
| --- | --- | --- |
| ai | backlog | idle only |
| ai | todo | idle only |
| ai | doing | idle, running, paused |
| ai | done | idle only |
| human | any (including backlog) | idle only |

- **Idle** means the task is not being worked on by an agent. **Running** is self-reported, not liveness monitoring. It is not reset automatically, apart from tasks run through an agent connection: such a task goes back to waiting when its connection has been inactive for too long, is replaced, or ends with its session. Agents do not change it just because a session started. **Paused** (`paused`) is a task in Doing whose run is stopped. A task becomes Paused when a question is asked or linked, when the resident agent stops on an error, or when a person pauses it, or when an agent pauses its own Running task because the stale-running reminder flagged it and the agent is not working on it. An agent uses `state <id> paused` for nothing else.
- `running` and `paused` exist only for an AI task in Doing; every other task is `idle`. `task.json` stores `idle`, `running`, or `paused`. Any other value, including `ran` and `awaiting_answer`, makes the file invalid like any unknown value.
- **Awaiting answer** is not a run state. It is a derived flag, `awaitingAnswer`, that is true when the task refers to a project question that has no reply yet, or to a message file that cannot be read. It applies to any assignee, any lane (Backlog and Done included), and any run state; archived tasks are excluded. It never overwrites `agentStatus`: readers derive it from the task's `questions` (see [Messages](./history-and-decisions.md#messages)), and the UI shows it over the run state label. The agent may continue other tasks meanwhile.
- **Backlog** (`taskStatus: backlog`) holds settled plans that must not be started yet. It means "do not start": a Backlog task is always Idle, and nextTasks, `wait`, the hook, the resident session, and automatic Done only consider Todo and Doing. See [Backlog and send](#backlog-and-send).
- Invalid combinations are never saved. A draft task is Idle in any lane.

How operations move a task between states:

- **Create** puts the task in Todo, Idle, at the end of the lane, and records a creation entry. Without a body, the product writes an empty `acceptance` and an empty body; a supplied body must pass frontmatter validation.
- **Lane move** requires `actor`; `reason` is optional. The task goes to the end of the target lane. A real move keeps the task's open questions and resets the run state to Idle when the task enters Doing or leaves Doing (to any lane), so a task moved into Doing starts Idle. A task that is already in Doing and is only reordered keeps its state. A real move appends history and updates `taskStatusChangedAt`. Moving to the same lane changes nothing. Automatic Done also sets Idle.
- **Run state change** requires `actor` and keeps the lane. `state` applies to an AI task in Doing. The only changes allowed are Idle to Running, Running to Paused, and Paused to Running; any other change is refused with `invalid_state_change` (for example Running to Idle or Idle to Paused). A value other than `idle`, `running`, or `paused`, such as `ran` or `awaiting_answer`, is refused with `invalid_value`. An agent may change a Running task to Paused with `--reason` saying why it stopped; the stale-running reminder is the only case where the skill tells it to, and it resumes with `state <id> running`. An AI change to Running is refused with `questions_unanswered` while the task waits for an answer. Questions are asked with `message ask`, which also pauses a Running task in Doing in the same save; `message link` does the same when it attaches a task (see [Messages](./history-and-decisions.md#messages)). Moving the lane or archiving keeps the task's questions and references. Re-reporting the same state is not a change.
- **Automatic resume.** When an agent takes in an answer (`message accept <Q-id> --actor ai`, or `message ask` and `message wait` returning an answered result, which accept it), each AI task in Doing that refers to that question, is Paused, is not a draft, and refers to no other open question goes back to Running in the same save when the task has no agent or its agent has an active connection that has shown activity within `STALE_RUN_SECONDS`. When the assigned agent has no such connection, the task stays Paused and no resume entry is written. The history entry is generated and localized: "Resumed after the answers to Q-1." (several IDs are joined with ", "). The pause that a question causes records "Waiting for the answers to Q-1."
- **Doing back to Todo.** An agent cannot move a task from Doing back to Todo: `mv <id> todo --actor ai` on a Doing task is refused with `human_only` ("moving a task from Doing back to Todo is for a person only."). A person may. An agent finishes a task by moving it to Done.
- **Edit** changes only the title, body, and acceptance conditions; the assignee never changes. CLI `--desc`/`--desc-file` take the whole file but keep the history and the `questions` refs of the latest stored `TASK.md`; those two keys in the input are ignored, so a task that refers to an open question stays awaiting an answer. `epic edit --document-file` replaces the whole `EPIC.md` text. The web UI sends `body` and `acceptance`, applied to the latest file under the lock.
- **Epic membership** sets or clears `epicId`; only real changes append history. People can change it on draft and archived tasks.
- **Delete** removes the task folder. The web UI deletes only archived tasks; the CLI asks for terminal confirmation.

Ordinary updates are last-write-wins: there is no content check or edit lock. Only answers, AI writes to a person's drafts, writes to archived tasks, and bulk-archive targets are validated against the latest `task.json` under the write lock.

## Epics and waves

An epic is a management unit that groups tasks and sets their work order. Its details (`EPIC.md`) are Markdown notes on its goal, scope, and decisions. A task belongs to at most one epic. The task's `epicId` is the only record of membership; the epic keeps no list of its tasks.

| Field | Rule |
| --- | --- |
| `id` | EPIC-ID: 1 to 64 characters matching `[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*`. Lowercase input is uppercased. Immutable. |
| `name` | 1 to 100 characters after trimming, one line. |
| `color` | `#RRGGBB`, uppercased. Random at creation when omitted. |
| `draft` | `null` or `"human"`, as for a task. A person's draft epic (`human`) is refused by CLI `epic edit` and `--waves` with `epic_is_draft`. Member tasks and `epic attachment` are not affected. An epic has no lane; `progress` has an optional `backlog` count. |
| `document`, `body` | The whole `EPIC.md` text, frontmatter included (0 to 100,000 characters), and the body without frontmatter. |
| `waves`, `wavesError` | The member tasks in work order as waves (a list of task ID lists), and whether reading the stored order failed. |

- Tasks store only `epicId`; the label color and name are resolved when read.
- An epic referenced by any task, archived ones included, cannot be deleted (`epic_in_use`). The web UI deletes an epic from the epic editor through `DELETE .../epics/{epicId}`; the CLI uses `epic rm`.
- A new project starts with the default epics `BUG` and `ENHANCEMENT`. They are ordinary epics: they can be edited and deleted. `defaultEpicsSeeded` in `project.json` is set once the project has been offered them, so a deleted default epic is not created again. `serve` offers them once to a project that does not have the flag yet.
- Epics have no managed acceptance conditions. Reference conditions are ordinary Markdown in the body; the API rejects `acceptance` input for epics.
- `waves` is a YAML list in the `EPIC.md` frontmatter in which every item is one wave: a task ID (`24`) or task IDs joined by commas (`26,25`; spaces after commas and quotes are accepted). Nested lists are not accepted. Duplicates across waves, non-positive IDs, empty items, and invalid types are errors. Writing drops empty waves. IDs that are no longer members are hidden and dropped on the next save, and a wave that becomes empty is dropped; unlisted members follow in creation order. Changing the waves does not count as editing a Done task. Lane positions never carry the order.
- A wave starts only when every task of the earlier waves is Done. `nextTasks` holds back a Todo task while any non-archived task of an earlier wave of its epic is not Done; archived tasks are ignored, tasks that no wave lists are never held back, and an unreadable order gates nothing. Stalled Doing work is still offered as `resume`.

## Drafts

A draft is a task or epic a person is still writing. `draft` is `null` (a normal task) or `"human"` (a person is writing it in the browser, an edit lock). The stored field is described in [Shared files](../reference/files.md). Drafts never expire, and `wait` and automatic Done skip every draft. There is no AI draft: a plan an agent proposes is a Backlog task (see [Backlog and send](#backlog-and-send)).

- A task or epic an agent creates (CLI `add` and `epic add`) has `draft: null`. What a person creates is `null`, or `human` when the browser creates it as a draft.
- The preview enters a person's draft at its first real change: `PUT /tasks/{id}/draft` with `draft: "human"` for a task, and `PATCH /epics/{id}` with `draft: "human"` for an epic. Only a task that is not Running can become a draft; Running is refused with `invalid_state_for_draft`. A real change appends history but not `updatedAt`. Entering or leaving an epic draft does not change `updatedAt` either. A client of an earlier version that sends `draft: "ai"` is accepted and the value is treated as `null` (the edit lock is released); creating a task with `draft: "ai"` is refused.
- A `human` draft: CLI `edit`, `mv`, `state`, and `history add` check `draft` under the write lock and fail with `task_is_draft`; CLI `epic edit` and `epic edit --waves` fail with `epic_is_draft`. Reads, `add`, `epic attachment`, and membership changes are not affected. The web `PATCH` on an epic does not refuse a draft.
- Web editing accepts only draft tasks. With `finalize`, the body, title, `draft: null`, and a history entry are committed as one pair. Discard sends `discard: true` with `draft: null` instead, and the history records `Discarded draft changes.` An epic has no history, so its discard is only `draft: null` on `PATCH /epics/{id}`.
- The product cannot stop direct edits of a draft's `TASK.md`; the README and the agent skill tell agents not to.

## Backlog and send

The Backlog lane holds settled plans that must not be started yet; it means the same as "do not start". Plans are corrected through chat or requests to the AI; there is no Backlog-specific editing, and normal card editing is unchanged. A Backlog `epic.json` has `formatVersion: 2`; a `task.json` is `formatVersion: 3` whether or not it uses the Backlog (see [Saving and recovery](./saving-and-recovery.md#format-versions-and-the-draft-mark)).

- **Create.** `agpj add ... --actor ai --backlog` creates a card in the Backlog; an agent uses it for a request for the plan only. Without `--backlog` an agent's `add` creates an ordinary Todo task (open questions make it Awaiting answer). `epic add` has no lane: an agent's epic is an ordinary epic, and a plan-only epic is one whose cards are created with `--backlog --epic <ID>`.
- **Send.** A person, or an agent on the user's instruction, sends Backlog to Todo: `agpj mv <id> todo --actor human|ai [--force]` for one card (there is no separate `send` command), and `agpj epic send <EPIC-ID> --actor human|ai [--tasks 1,2] [--force] [--reason TEXT]` for a whole epic or selected cards, in one all-or-nothing save. The web routes are `POST /tasks/{id}/send` and `POST /epics/{epicId}/send` (see [Web API](../reference/web-api.md)); the web UI sends from the Backlog lane, a card's details, and an epic's details (see [Server and UI](./server-and-ui.md)). The skill has an agent send only when the user told it to and record that in `--reason`; `--force` from an agent gets `human_only`. A card sent to Todo is put at the end of Todo. `mv` applies these rules only when it moves a card from the Backlog to Todo; a card already out of the Backlog is moved like any other, and the send route changes nothing for it, so a retry is safe. A card a person is editing (`draft: human`) cannot be sent (`task_is_draft`). The history records `A person sent the task to Todo.` or `An agent sent the task to Todo.`, followed by the `--reason` when one was given.
- **Open questions.** A plan with open questions is refused with `send_with_open_questions` (409; the message lists the question IDs) until the person confirms with `--force` (CLI) or `ack: true` (API). After the send the question stays and the card is Awaiting answer until it is answered. Awaiting answer applies in the Backlog too, so a plan with open questions already shows it before the send.
- **Moves.** An agent's `mv` and `state` on a Backlog card fail with `task_in_backlog`. A person's move to Todo, from the CLI or the Web, is a send; a move to Doing or Done is allowed for a person too and follows the rules of a send (open questions need `--force` or `ack`, a card in editing is refused). An agent never moves a Backlog card to Doing or Done; its only way out of the Backlog is the send. `agpj mv <id> backlog --actor human` takes an unstarted Todo card (Idle, no results) back to the Backlog; an agent gets `human_only`.
- **Agent surfaces.** nextTasks, `wait`, the hook, the resident session, auto-Done, and the progress table candidates ignore the Backlog. A Backlog card in an epic still keeps later waves waiting, because it is not Done. The epic's `progress` has an optional `backlog` count, left out when 0.
- **Rules in one place.** The rules for a send are in `src/file_store/ops/send.rs`; the CLI's `mv <id> todo` and the Web send route both end in its `send_task_locked`, and its `check_sender` and `open_questions` refuse an agent and an unconfirmed plan with open questions, for the task or for every card the epic send would move.
- The product cannot stop direct edits of a draft's `TASK.md`; the README and the agent skill tell agents not to.

## Archive

- Archiving keeps the lane, `draft`, `epicId`, and position; sets the run state to `idle`; keeps the task's question references; and updates `updatedAt`. The lane and `taskStatusChangedAt` do not change. Web archive and restore record the fixed reasons `Archived by user.` and `Restored by user.`
- Restoring places the task at the end of the lane it was archived from. The run state stays `idle`, so the lane and the state are always a valid pair under the work model. The phone's archive list restores through the same `POST …/tasks/{id}/restore` as the PC.
- Normal snapshots, `ls`, and the hook return unarchived tasks only; `show ID` also shows archived ones. Writes to archived tasks fail with `task_archived`, but their images remain readable. The archive list is sorted by `archivedAt`, newest first, with the higher task ID first for equal times and tasks without a readable `archivedAt` last. `GET …/archive` sorts with `sort_archived_newest_first` (`src/file_store/reader/snapshot.rs`), and the web UI applies `sortArchivedNewestFirst` (`web/src/features/tasks/api/read.ts`) again, so the PC list and the phone's list share one order.

## Planned versions

A project lists versions as files and can name one of them as its release version, and a task can be planned for one version. The plan is optional: a task without one stays without one. The versions are the `*.md` files directly inside `docs/release/` of the project's Docs, the project keeps `releaseVersion` in `project.json`, and each task keeps `plannedVersion` in its `TASK.md` frontmatter. See [Planned versions](../reference/files.md#planned-versions) for the layout, the order, and the format of a version name.

- The planned version of each task, the `plannedVersion` key of its `TASK.md`, is the source of truth; a person can edit it by hand. It is set per task or for every task of an epic in one call, and one epic can hold tasks of different versions.
- Assigning the release version stores its name at that moment. Changing the project's release version later moves no task.
- A change appends one ordinary history entry in the same save, which records the version before and after and the time, so a later reader can tell which tasks left a version and when. Setting the version a task already has changes nothing, so a retry adds no entry.
- The operations (`set_task_planned_version`, `set_epic_planned_version`, `add_version`, `set_release_version`) run under the project's write lock like other writes. Setting a version never changes the assignee, lane, run state, position, or acceptance conditions. It is the one field that stays open on a Done or archived task, for an AI as well as a person (every other edit of a Done task stays read-only for an agent, and an edit that combines a version change with another change on a Done task is refused as a whole); an AI cannot change a person's draft. The bulk epic operation still skips archived and Done tasks.
- The Summary carries `project.versions`, `project.releaseVersion`, and each task's `plannedVersion` (see [Web API](../reference/web-api.md#summary-and-detail)). Adding or removing a version file by hand, or editing `plannedVersion` in `TASK.md`, reaches the board like any other file change.

## Ordering and time

- On the desktop board and in the CLI, Todo and Doing sort by ascending position and Done by descending position. The mobile Done list sorts by `taskStatusChangedAt`, newest first. Completion order is never inferred from history. API `tasks` come in ID order; the UI sorts them for display. Epics sort by EPIC-ID.
- Stored and JSON times are UTC strings in exactly `YYYY-MM-DDTHH:MM:SSZ`; offsets (even `+00:00`) and fractional seconds are rejected. The CLI displays local time of the machine, the UI that of the browser, as `MM/DD HH:mm`.
