# Brainstorm mode: deepen one epic or task with the user

The project has a mode, shown as `mode` in `agpj progress --json` and in the hook's table. When `mode` is `brainstorm`, the user wants you to ask questions that deepen one epic or one task, the target. The table then also has `modeTarget`: `{"type": "epic" | "task", "id": "<EPIC-ID or task number>"}`. Read the mode again after context compaction and before every question, because the user can end it at any time.

## Start

Start when a request that begins with "Start brainstorming on ..." arrives, or when the progress table shows `mode: "brainstorm"` and you have not asked yet. Do not wait for the user's first question.

1. **Read the target.** For an epic, read its `EPIC.md` (the epic's `documentPath` in the progress table) and its tasks. For a task, read its `TASK.md` (the task's `taskPath`), its acceptance conditions, and its history. `agpj --project "<name>" epic show <EPIC-ID> --json` and `agpj --project "<name>" show <task-id> --json` print the same.
2. **Find what is already decided.** Read the history and the results of the task or of the epic's tasks (`show <task-id> --json`), and the answered questions (`agpj --project "<name>" message ls --json`). Do not ask again what is already answered.
3. **Ask one question** about the biggest gap you found: the purpose, the reader or user, the constraints, the alternatives, or the risks.

## Ask one question at a time

```sh
agpj --project "<name>" message ask --actor ai --brainstorm --questions-json '{"questions":[{"body":"Who reads this page first, and what should they do next?"}]}'
```

- Use `--brainstorm` and no `--task`: a brainstorm question belongs to the target, and a task link would pause the task.
- Ask **one** question per `message ask`, then wait for the answer as `agpj skills questions` says.
- When choices help, give 2 to 10 of them with a `recommendedChoiceId` and say why in the body.
- Follow up on each answer with the next point. Stay on the target; if a question is about ordinary work, ask it without `--brainstorm`.
- Record a decision the user reaches. On a task, use `agpj --project "<name>" history add <task-id> --reason "<the decision and why>"`. For an epic, add it to the epic's `EPIC.md` in the way `agpj skills files` describes.

## When it ends

The user ends the brainstorm with `agpj mode sparring` or on the board (`agpj mode auto` returns to the schedule, or to sparring). Then `mode` is no longer `brainstorm`: stop asking brainstorm questions at once and go back to `agpj wait`. `message ask --brainstorm` is refused with `brainstorm_not_active` once the mode has ended: stop, do not retry, and check `mode` in the progress table. A request "Start brainstorming ..." that arrives after the mode has ended is only accepted: `agpj message accept <id> --actor ai`.

Sleep always wins. When a sleep window starts, the mode becomes `sleep`: `message ask` only holds questions until sleep ends, so ask nothing that a subagent can decide and follow `agpj skills sleep`.

Questions asked without `--brainstorm` stay ordinary questions and are accepted in brainstorm mode, so work questions still reach the user.
