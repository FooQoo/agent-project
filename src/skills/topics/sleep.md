# Sleep mode: decide without the user

The project has a mode, saved per project and shown as `mode` in `agpj progress --json` and in the hook's table. When `mode` is absent the project is in sparring (the default): ask the user with `message ask` as `agpj skills questions` says. When `mode` is `sleep`, the user is away on purpose and does not want to be asked anything. Read the mode again after context compaction and before every question, because the user can switch it at any time (`agpj mode` shows it, `agpj mode sparring` and `agpj mode sleep` switch it; the board has the same switch).

In sleep mode `message ask` is refused with `sleep_mode` and nothing reaches the board, the notifications, or the chat. Do not ask in the chat instead, do not wait, and do not stop the work. Handle the refusal, and any decision that you would have asked about, like this:

1. **Delegate the decision to a subagent.** Start a subagent in your agent (the Task or Agent tool of Claude Code, a spawned agent in Codex) and give it the question, the choices with your recommendation, the constraints (the task's plan and acceptance conditions, what must not break), and where to look. Name it so the history can tell who decided: `subagent:<name>`, such as `subagent:release-reviewer`.
2. **Ground the decision in what exists.** Tell the subagent to base its decision on existing code, the project's docs (the product docs under the project's `docs/` in the shared folder, and the repository's own docs), and past questions and answers of the board (`agpj message ls --json`, `agpj message show <Q-id> --json`, or the answered questions of the project's `qa/` folder). A past answer to the same or a similar question wins over a new guess. It returns the choice, why, and the grounds it used (file paths, document names, `Q-<n>` numbers).
3. **Take the conclusion and continue.** Do not ask the subagent's conclusion back to the user and do not wait. Continue the task with it. If the decision is irreversible (deleting data, publishing, spending money, sending a message outside the project), choose the safest reversible option instead, or leave that one step undone and say so in the record.
4. **Record it on the task**, so the user sees it when they return:

```sh
agpj --project "<name>" decision add <task-id> --question "<what would have been asked>" --choice "<the choice taken>" --reason "<why>" --decided-by subagent:<name> --basis "<code path, document, or Q-123>" [--basis "<another ground>"]
```

`--basis` is required: give at least one ground, and repeat the flag for each. The command adds one entry to the task history in the form `Decided in sleep mode by subagent:<name>. Question: ... Choice: ... Reason: ... Basis: ...`, which the board shows in the card's history. Record each decision on the task it affects; record it before you act on a risky one.

When the user turns sleep mode off, you are back in sparring: ask again as usual. Decisions made in sleep mode stay in the history. If the user disagrees with one on return, they say so and you change course.
