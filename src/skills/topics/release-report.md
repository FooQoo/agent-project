# Report a version and look back at a theme

Use this when the user asks what a version delivered, what is left, or how a theme (a goal that spans several versions or epics) has advanced. Every statement you write must trace to a card, a result, or a history entry. Never write from memory of the conversation.

## Per-version report

1. Generate the base: `agboard --project "<name>" release doc --version <V>` (or `--current-version`). It writes `docs/release/<V>.md`; the block between the `agboard:generated` markers is the evidence. Write your own text outside the markers: the summary, the reasons for moves, the unconfirmed list. Regenerating keeps it.
2. The block already covers what a hand search misses: cards moved off the version are found from their history, each once, with where they went; archived cards count; Verified means every acceptance condition is done.
3. Open `agboard --project "<name>" show <id>` for any card you describe in more than a line. The report names it in `Source:`.
4. Publication is separate from Done. A card counts as published only when one of its results has a line `Published: <V>` in its summary and a link to what was published (release page, package, merged pull request). Without a link the claim is unconfirmed. Done alone is never publication.
5. Say partly confirmed outcomes plainly: "3 of 5 planned cards have a publication record; #12 is Done but has none; #15 claims it without a link."

To record the publication of a card, after the release is really out:

```sh
agboard --project "<name>" result add <id> --actor ai --final --title "Released in <V>" \
  --summary "Released.
Published: <V>" --link https://example.com/releases/<V>
```

A Done card is read-only for agents, and a card that is Done and already has a final result takes no new result from anyone. So record the publication result while the card is still open, or ask a person to move the card out of Done, add the result, and move it back. Do not work around it by editing `task.json`. Never record `Published:` for something you did not see published; when the release is not out yet, report the card as unconfirmed.

## Theme retrospective

A theme is whatever the user names; the board has no theme field. Find its epics and cards first (`epic ls`, `epic show`, `ls --epic <ID>`, `ls --version <V>`, titles and bodies), and state which ones you took in and why.

1. Read each epic's completion conditions (`epic show <ID>`). These, not the card count, are what the theme is judged on.
2. For each condition, find the cards that bear on it and read their acceptance conditions, their `Verification` text, results, and history, across all versions (include archived cards and cards that moved between versions).
3. Mark each condition: verified (a card's verification says it directly), advanced but not met (name what is open), not started, or unconfirmed (evidence missing or only claimed). Name the card and result for every mark.
4. List what advanced, version by version, using the per-version reports, and list the unconfirmed items with what would confirm each.
5. Never infer a percentage, a completion rate, or "almost done" from counts of cards or conditions. Cards differ in size and conditions are not cards. Do not check an `EPIC.md` condition yourself; a person who owns it confirms it.

Write the result to the Docs (for example `docs/retrospective/<theme>.md`) with the same shape: scope and sources, a table per epic (condition, status, evidence), what advanced by version, unconfirmed, not claimed, next. A worked example is in the user guide page "Version reports and theme retrospectives".
