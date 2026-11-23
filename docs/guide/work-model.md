---
title: The work model
---

# The work model

Agent Project is built around a model of the work, not around a screen. The model lets an agent know what to do next, when it is finished, when to ask you, and what to leave behind. The board, the `agpj` command, and the files are all ways into the same model. Read this page once you have tried the loop; the other pages build on it.

## The elements

| Element | What it is | What it does for the agent |
| --- | --- | --- |
| **Project** | One board, bound to the folder you work in. It has its own epics, tasks, questions, and history. | Fixes the scope of the work. If no project is found, the agent stops and tells you rather than guessing. |
| **Epic** | A goal, with an order for its tasks. | Gives the order in which to take the tasks. |
| **Task** | One piece of work that can be reviewed on its own, owned by **AI** or **User**. The owner is fixed when the task is created. | Sets the unit of work and who does it. |
| **Acceptance conditions** | A checklist on each task that says when it is finished. | Is the test for being finished. |
| **Question** | A decision the agent asks you for, usually with choices and a recommendation. | Makes the agent stop and wait. |
| **Request** | A message you write for the agents. | Hands new work to an agent. |
| **History** | A time-ordered list of what happened, each entry with a time and a reason. | Lets a new session, or another agent, continue from where the last one stopped. |
| **Draft** | A task or epic you are still writing in the browser. | Keeps agents from working on it. |
| **Backlog** | A lane for settled plans that must not be started yet. | Means "do not start": agents never start it, and it reaches Todo only by a send: you send it, or an agent does when you tell it to. |
| **Archive** | Tasks you have put away. | Stops all writes to them. |

A task also has a **title** and a **body** in free Markdown for the plan, the results, and links. A task belongs to at most one epic, and the epic's order and the task's place in a column are separate things.

Everything is stored as plain files in a folder on your machine (by default `~/.agent-project/`). The browser, the `agpj` command, and the agents read and write the same files. See [Data and backups](./operations/data).

## States

A task is in one of three **columns**: **Todo** for work that has not started, **Doing** for work in progress, and **Done** for finished work. An AI-owned task in Doing also shows what its agent is doing: **Idle**, **Running**, or **Paused**.

| Owner | Todo | Doing | Done |
| --- | --- | --- | --- |
| **AI** | Idle | Idle, Running, Paused | Idle |
| **User** | Idle | Idle | Idle |

- **Idle** means the agent is not working on the task.
- **Running** is what the agent last reported, and it applies to AI tasks in Doing only. It is not a live check: if an agent crashes, its task stays Running until someone changes it.
- **Paused** means the task is in Doing and its run has stopped. A task becomes Paused when an agent asks a question while running (see below), when the resident agent stops on an error, when you pause it, or when an agent that the stalled-task reminder flagged is not working on the task and pauses it.
- In Doing, the state changes only from Idle to Running, from Running to Paused, and from Paused to Running; anything else is refused.
- **Awaiting answer** is not a state and is not set by anyone. It is worked out from the questions: any task, in any column and of any owner, shows Awaiting answer while it refers to an open question. The label replaces the state label, and a task that is not in Doing shows no state label otherwise.
- Moving a task into Doing or out of Doing (to any column) resets its state to Idle, so a task moved into Doing starts Idle. Reordering a task inside Doing keeps its state. Moving a task keeps its open questions.
- Tasks you own are always Idle.

## Questions {#decisions}

When an agent cannot continue without you, it asks you a **question**, usually with a few options and a recommendation. Questions belong to the project, and the task that refers to one shows **Awaiting answer** and shows the question. You choose an option, write a note, or both; the answer goes into the history and the agent reads it before continuing. A question stays open until it is answered.

You can answer on the board, on your phone, in the chat, or with the CLI. The first answer recorded is the one that counts. [Decisions](./delegate/decisions) explains how to answer.

## Acceptance conditions and automatic Done

Acceptance conditions are written before the work starts. An agent marks a condition done only after it has verified it. When every condition of an AI-owned task is done and nothing is waiting on you, the task moves to **Done** by itself, while the server is running, and the history records why. Tasks without conditions, tasks you own, drafts, and Awaiting answer tasks do not finish automatically. When a task finishes, its state is Idle. The file format is described in [Markdown file format](./reference/file-format).

## Epics and the order of work

An epic is a management unit that groups tasks. It has a name, a color, details written in Markdown, and an order for its tasks. Agents read the order to choose what to do next. See [Epics and progress](./watch/epics).

## History

Every change adds an entry to the task's history: moves, state changes, questions and answers, checked conditions, and the progress notes that agents add. Each entry has only a time and a reason, so the history reads like a log. The agent also writes down what it did and what it verified. The History tab shows it, newest first.

## What Agent Project guarantees

These rules are kept by Agent Project itself, not left to the agent's good manners:

- **Who owns a task never changes.** The owner is fixed when the task is created.
- **Only valid states exist.** A column and a run state that do not go together are refused, and nobody can set a state that does not fit. In Doing, only Idle to Running, Running to Paused, and Paused to Running are allowed.
- **The agent waits when you have not answered.** A task that refers to an open question shows Awaiting answer, and the agent cannot mark it Running. Asking a question about a Running task in Doing, or linking one, pauses it at the same moment and records "Waiting for the answers to Q-1." in its history. When the agent takes in your answers, the task goes back to Running by itself and the history records "Resumed after the answers to Q-1." An agent also cannot move a task from Doing back to Todo; you can.
- **An answer is recorded once.** The first answer stands and is not overwritten. A question, its answer, and the history of the tasks that refer to it are saved together.
- **Done means the conditions are met.** An AI-owned task needs at least one acceptance condition, all of them done, and no open question. The check is made again on the latest files at the moment it moves, and the reason goes into the history. This is what automatic Done checks. An agent that moves a task to Done by hand also needs every condition done and no open question, but a task with no conditions is not refused; see [What still relies on the agent](#relies-on-agent).
- **Finished work stays finished.** An agent cannot change a Done task. If you move it back, it can be worked on again. A Done task does not go back by itself when you uncheck a condition.
- **Drafts and archives are fenced off.** An agent cannot touch your drafts, and nobody can write to an archived task.
- **Everything leaves a reason.** Changes to columns, states, questions, answers, and checked conditions are recorded in the history with their reasons. Task and question numbers are never reused.
- **A broken record is not guessed at.** A malformed history is reported as an error rather than read loosely.
- **Rewriting a plan keeps the record.** Replacing a task's body and conditions keeps its history and its links to questions.

## Where you step in

There are two ways to take part, and both go through the same work model.

| Way | What happens | What you do |
| --- | --- | --- |
| **Answer when asked** (human in the loop) | The agent stops and waits for a decision it cannot make. | Answer a question the agent asks during the work. |
| **Watch and step in** (human on the loop) | The agent keeps working while you look. It follows your change the next time it reads the board. | Move a card back or to Done, check a condition, rewrite a body or conditions, archive, or delete. |

The agent calls you only in four cases: your intent is unclear, your preference decides it, an action cannot be undone, or the work grows bigger than you asked. This line is part of the agent's Skill. Agent Project refuses to mark a task Running while its questions are open, but it does not block the agent's other actions; the agent stops because its Skill tells it to. See [Working with agents](./delegate/agents).

## What still relies on the agent {#relies-on-agent}

Some things are not enforced by Agent Project yet. They depend on the agent following its Skill, so know where they are:

- **Tasks with no conditions.** An agent could move a task without any acceptance conditions to Done by hand. Automatic Done never does this.
- **Who answered.** Agent Project does not check who actually typed an answer. When you reply in the chat, the agent records your words on the board.
- **Editing the files directly.** Changing `TASK.md` by hand skips these checks and the automatic history. Use the board or the CLI.
- **Running.** Running is self-reported, so a task can look active when its agent has stopped.
- **Automatic Done needs the server.** It happens only while `agpj serve` is running, so a finished task can wait in Doing until it is.
- **Your own tasks.** Tasks you own are recorded, not verified.
- **Epics.** Agent Project does not track whether an epic is finished; its completion conditions are written in its body.

## How it differs from AI-DLC and spec-driven development {#compare}

AI-DLC and spec-driven development (SDD) answer a different question from the work model. They are methods: they say what to produce, and in which phases, before and while an agent writes code. Agent Project is a harness: it is the place where the work runs and is tracked, whatever method produced the plan.

[AI-DLC](https://aws.amazon.com/blogs/devops/ai-driven-development-life-cycle/) (the AI-Driven Development Life Cycle, introduced by AWS) has the AI draft plans, code, and tests while people validate each step. Its workflow runs from inception, where requirements and units of work are agreed, through construction to operations, and AWS publishes it as [open-source workflow rules](https://github.com/awslabs/aidlc-workflows) for several coding agents. SDD writes a spec before the code and treats it as the source of truth for the person and the agent. [GitHub Spec Kit](https://github.com/github/spec-kit) walks from a constitution through specify, plan, and tasks to implement, and [Kiro](https://kiro.dev/docs/specs/) keeps `requirements.md`, `design.md`, and `tasks.md` for each feature.

Agent Project does not prescribe phases or documents. It adds what a method alone leaves to convention: one shared board that holds the plan, the progress, and the history, and rules that Agent Project itself enforces. The agent waits for an open question, a task finishes only when its acceptance conditions are met, and the history lets another session or agent continue. Because it works through a CLI, a Skill, and a hook, it is not tied to one agent; the plugin covers Claude Code and Codex.

The approaches combine well. A spec written with Spec Kit or Kiro, or the phase plan of AI-DLC, can live in an epic, with each task of the plan as a task on the board and each acceptance criterion as an acceptance condition. The method decides what to build, and Agent Project carries the work through to Done.

|  | Agent Project | AI-DLC | Spec-driven development |
| --- | --- | --- | --- |
| **What it is** | A harness: a local board and rules that the work runs on. | A development method with phases, centred on AI-drafted plans that people validate. | A practice of writing a spec before code, supported by tools such as Spec Kit and Kiro. |
| **What it produces** | Epics, tasks, questions, acceptance conditions, and a history, as plain files. | Requirements, units of work, designs, code, and tests, phase by phase. | A spec, a plan, and a task list as Markdown files, often one set per feature. |
| **Where plan, progress, and history live** | One shared board for the project, with a history in every task. | In the files and the session of the workflow in use. | The plan and task list live in spec files; progress depends on the tool. |
| **How you step in** | You answer questions, or watch the board live and move cards, check conditions, or rewrite plans, from the browser or your phone. | You validate the AI's proposals at each phase. | You review and edit the spec files. |
| **How done is judged** | Automatically: all acceptance conditions are done and no question is open. | By the validation of each step. | By the tasks and checklists in the spec; the tool does not verify them. |
| **Resuming in another session or agent** | The task history records what was done and why, so a new session or agent continues from there. | Relies on the workflow's recorded state. | Relies on the spec files. |
| **Tool dependence** | Any agent that can run the `agpj` command; the plugin covers Claude Code and Codex. | Rules for agents such as Claude Code, Kiro, and Codex CLI. | Depends on the tool, for example an IDE or a set of slash commands. |

These columns describe each approach at its core, as its public documentation presents it. Teams adapt methods and tools, so check the sources above for the details. The strengths of Agent Project are in its column, the first after the labels: its guarantees are enforced rather than agreed, and the person and the agent see the same state as it changes.

## How it differs from Vercel Board and Backlog.md {#compare-boards}

Vercel Board and Backlog.md are also places where people and agents share work, so they sit closer to Agent Project than the methods above do. Agent Project is a project-management harness for agents: it is not only a place that shows tasks, but a tool that enforces rules. Agents break work into epics and tasks with acceptance conditions, ask you on the board and wait for the answer, and record a history so another session can continue. A task becomes Done automatically only when its conditions are met and no question is open.

[Vercel Board](https://vercel.com/blog/introducing-vercel-board) ([docs](https://vercel.com/docs/board)) is an experimental message board for a team's people and coding agents. It organizes channels, threads, and labels, and a task is a thread with a state that the board keeps. Agents claim tasks, post updates, and wait for replies, and a thread can sync with Slack, GitHub, or Linear. [Backlog.md](https://github.com/MrLesk/Backlog.md) is a Markdown-native task manager and Kanban board for a Git repository. Each task is a Markdown file in the repository, with acceptance criteria and an optional Definition of Done checklist, and it is used from a CLI, a terminal board, or a local web board.

|  | Agent Project | Vercel Board | Backlog.md |
| --- | --- | --- | --- |
| **What it is** | A harness: a local board and rules that the work runs on. | An experimental message board with channels, threads, labels, and tasks for a team and its agents. | A Markdown-native task manager and Kanban visualizer for a Git repository. |
| **Where the data lives** | Plain files in a local data area on your machine. | On your Vercel team's board, with data stored in the United States. | Markdown files in the repository, with no server, account, or telemetry. |
| **How work is organized** | Projects, epics, and tasks in lanes; the agent plans the breakdown. | Boards, channels, and threads; a task is a thread, and tasks can depend on other tasks. | Tasks as files, with statuses, milestones, and dependencies. |
| **Completion conditions** | Each task has acceptance conditions that the work must meet. | Not described; an agent completes a task with a message. | Acceptance criteria per task, and an optional Definition of Done checklist. |
| **Asking a person** | The agent asks on the board and waits for your answer. | Agents can wait for replies from people or other agents. | Not described; the README describes review checkpoints for the spec, the plan, and the code. |
| **Done** | Automatic: all acceptance conditions are done and no question is open. | An agent marks the task complete. | Tasks move through statuses, with a configurable final status; the README does not describe an automatic check. |
| **History for another session** | Every task keeps a history of what was done and why. | A thread keeps the conversation and updates, and an event log records each change. | Not described. |
| **How you use it** | A web board, a CLI, and a skill for agents; the plugin covers Claude Code and Codex. | The `vercel board` CLI, a JavaScript SDK, and a skill; sync with Slack, GitHub, and Linear. | A CLI, a terminal board, a local web board, and an optional MCP connector. |

These columns compare each product as its public documentation describes it; "not described" means the pages do not say, not that the feature is absent. Vercel Board is experimental and may change. The products also fit different needs: Vercel Board suits agent swarms that coordinate on a team's Vercel account, and Backlog.md suits a task list that lives in the repository. What sets Agent Project apart is that its guarantees are enforced by the tool rather than agreed in convention.

## Next

- [Quickstart](./quickstart): delegate a piece of work and see these rules at work.
- [Working with agents](./delegate/agents): how an agent plans, asks, and finishes.
- [The board](./watch/board): watch and step in from the browser.
