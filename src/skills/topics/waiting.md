# Wait and resume

When no work can proceed in the ordinary work loop, wait for a change using the `cursor` of the latest snapshot of the same project. This wait can continue indefinitely; `--timeout` or Ctrl+C ends it without changing tasks or questions:

```sh
agpj --project "<name>" wait --after "<cursor>" --json
```

Never poll the board on a timer. `wait_timeout` (the end of an optional limit) and `board_unavailable` (a technical failure) are not answers. A waiting command starts the server itself; never start `agpj serve` first. `local_network_denied` means this environment does not allow local network connections: tell the user its reason (in Codex, `agpj setup` allows `agpj` outside the sandbox) and make no board-dependent changes until fixed. A question remains open until it is answered or withdrawn with `message withdraw <Q-id> --actor ai`. `cursor_invalid` and `project_not_found` are errors too: report them and make no board-dependent changes.

After `wait` or `message ask` returns, read the complete snapshot and history, including the user's reason and the answers printed by `message ask`. Taking in the answer (`message accept`, or the answered result of `message ask` and `message wait`) sets a paused task back to `running` automatically when no other question it refers to is still open, so you do not resume it yourself; re-read the task and continue only if it resumed. Reconsider the plan in light of the answer and update it if needed. If the task is not `running`, decide from its lane and history. While another question it refers to is still open (`awaitingAnswer` is true), keep waiting for that answer. If it is no longer in Doing, a person moved it: do not request `running`, and continue with the work loop. If the history shows that a person paused it after the answer, respect that and leave it as it is. Otherwise, when it is in Doing with no open question, set it with a reason, which the state operation records:

```sh
agpj --project "<name>" state <id> running --actor ai --reason "<how the answer changes the work>"
```

If the answer changes the scope or completion conditions, update `TASK.md` (and `EPIC.md`) before resuming; if information still conflicts, ask again.

For an explicitly requested serial listener, use the listener procedure in `agpj skills listening` and `wait --until task --idle-timeout <seconds> --json`. It is distinct from waiting for a change or for answers with `message ask`; never start it implicitly.
