# Wait and resume

When no work can proceed in the ordinary work loop, wait for a change using the `cursor` of the latest snapshot of the same project. This wait can continue indefinitely; `--timeout` or Ctrl+C ends it without changing tasks or questions:

```sh
agpj --project "<name>" wait --after "<cursor>" --json
```

Never poll the board on a timer. `wait_timeout` (the end of an optional limit) and `board_unavailable` (a technical failure) are not answers. A waiting command starts the server itself; never start `agpj serve` first. `local_network_denied` means this environment does not allow local network connections: tell the user its reason (in Codex, `agpj setup` allows `agpj` outside the sandbox) and make no board-dependent changes until fixed. A question remains open until it is answered or withdrawn with `message withdraw <Q-id> --actor ai`. `cursor_invalid` and `project_not_found` are errors too: report them and make no board-dependent changes.

After `wait` or `message ask` returns, read the complete snapshot and history, including the user's reason and the answers printed by `message ask`. Taking in the answer (`message accept`, or the answered result of `message ask` and `message wait`) sets a paused task back to `running` when no other question is open and the assigned agent has a fresh connection. Then follow the section below.

## After an answer

This is the single rule for what to do once an answer has been taken in; other pages point here. Re-read the task (snapshot and history), then decide from the first case that applies:

- `awaitingAnswer` is true, whatever the run state: another question it refers to is still open. Keep waiting for that answer only and do not continue the task meanwhile. Other independent work may go on.
- The task is a draft, archived, owned by a person, or not in Todo or Doing (it is in Backlog or Done): it is not yours to run now. Leave it unchanged and go on with other work.
- It is in Doing and `running`: continue the work.
- It is in Doing and `paused` or `idle`: if the history shows that a person paused it, or changed it, after the answer, leave it unchanged. Otherwise, from a session connected to its assigned agent, set it to `running` with a reason, which the state operation records:

```sh
agpj --project "<name>" state <id> running --actor ai --reason "<how the answer changes the work>"
```

- It is in Todo and `idle`: it had not started. Start it as the work loop describes (`agpj skills`, section 6), respecting epic order.

Then reconsider the plan in light of the answer. If the answer changes the scope or completion conditions, update `TASK.md` (and `EPIC.md`) before continuing; if information still conflicts, ask again.

For an explicitly requested serial listener, use the listener procedure in `agpj skills listening` and `wait --until task --idle-timeout <seconds> --json`. It is distinct from waiting for a change or for answers with `message ask`; never start it implicitly.
