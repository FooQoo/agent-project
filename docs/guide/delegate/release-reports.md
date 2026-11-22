---
title: Version reports and theme retrospectives
---

# Version reports and theme retrospectives

Use this page when you ask an agent what a version delivered, what is left, or how a goal that spans several versions has advanced. The agent builds both reports from the board, so every statement can be traced back to a card, a result, or a history entry. The skill it follows is printed by `agboard skills release-report`.

## A report for one version

Ask for a report of a version. The agent runs [`release doc`](../reference/cli#generate-a-release-document) for it, which writes `docs/release/VERSION.md`, and then writes the summary around the generated block.

The generated block lists the cards planned for the version, those verified, those done without acceptance conditions, those remaining, and those moved to another version, each with the card number and a pointer to `agboard show <id>`. Three points keep it honest:

- A card that left the version is found from its history and listed once, with where it went. Archived cards count.
- Verified means the card has acceptance conditions and every one is done. A Done card with no acceptance conditions is not verified and is not left in Remaining either: it is listed apart as done without acceptance conditions, because nothing says whether it met its goal.
- Moved to another version lists every card that left the version, whether it went to a later version, an earlier one, or to no version; each line names where it went.
- Published is a separate fact, described next.

### What counts as published

A card Done is not a card published. Agent Board counts a card as published for a version only when one of its results has a line `Published: VERSION` in its summary and a link to what was published, such as a release page or a package:

```sh
agboard --project "My project" result add 12 --actor ai --final --title "Released in v1.0.0" \
  --summary "Released.
Published: v1.0.0" --link https://example.com/releases/v1.0.0
```

A result with the line but no link is only a claim, and the report lists the card as not confirmed. A card with no such result is not confirmed either, however finished it is.

The explicit form checks the version name, requires the link, and writes the `Published:` line for you:

```sh
agboard --project "My project" result add 12 --actor ai --published v1.0.0 \
  --link https://example.com/releases/v1.0.0 --summary "Released."
```

A publication record can be added at any time, including to a Done card and to a card that already has a final result. It is the only change a Done card takes: other results, the body, the acceptance conditions, and the column stay as they are. The Web API accepts the same record with `POST /api/projects/{projectId}/tasks/{id}/results`; see the [Web API](../../dev/reference/web-api).

### Example: publication confirmed for part of a version

This block comes from a demo project in which one of the two verified cards has a linked record and the other only claims publication:

```markdown
## Release status of v1.0.0

- Planned: 2
- Verified: 2
- Done without acceptance conditions: 0
- Remaining: 0
- Moved to another version: 1
- Published (with a publication record): 1 / 2

### Planned

- #3 Queue edits while offline (done; acceptance 1/1)
- #5 Open the last board without network (done, archived; acceptance 1/1)

### Verified

- #3 Queue edits while offline (done; acceptance 1/1)
  - Source: `agboard show 3`, results 1, history entries 6
  - Result: Offline queue released [final] (https://example.com/releases/v1.0.0)
- #5 Open the last board without network (done, archived; acceptance 1/1)
  - Source: `agboard show 5`, results 1, history entries 6
  - Result: Offline board cache done [final]

### Done without acceptance conditions

No done task lacks acceptance conditions.

### Remaining

Nothing remains.

### Moved to another version

- #4 Merge conflicting edits on reconnect (todo; acceptance 1/2)
  - Source: `agboard show 4`, results 0, history entries 4
  - Moved to: v1.1.0

### Publication

A task counts as published only when one of its results states `Published: VERSION` on its own line and links to what was published. Done alone is not publication. Generating this document publishes nothing.

### Published

- #3 Queue edits while offline (done; acceptance 1/1)
  - Record: `agboard show 3`, result 1 (https://example.com/releases/v1.0.0)

### Not confirmed as published

- #5 Open the last board without network (done, archived; acceptance 1/1)
  - Unconfirmed: result 1 states the publication but has no link
```

The report says "1 / 2" because only #3 is backed by a record. #5 is Done and verified, but nothing links to its publication, so the report does not call it published.

## A retrospective for a theme

A theme is a goal the board has no field for, such as "offline support". Name it to the agent. It finds the epics and cards that belong to it, says which ones it took in, and then compares:

1. The completion conditions of each epic (`agboard epic show ID`).
2. The acceptance conditions, verification text, results, and history of the cards that bear on each condition, across versions and including archived cards and cards that moved.

Each condition is marked verified, advanced but not met, not started, or unconfirmed, and each mark names its card and result. The retrospective lists what advanced by version and what is unconfirmed with what would confirm it. It never gives a percentage or a completion rate from counts of cards or conditions, and it does not tick a condition in an `EPIC.md`; the person who owns the condition does.

### Example: a theme across two versions and two epics

This is the retrospective an agent wrote from a demo project. The theme is "Offline support", held as the epics `SYNC` and `CACHE`, over v1.0.0 and v1.1.0:

```markdown
# Retrospective: Offline support (v1.0.0 to v1.1.0)

Scope: the theme "Offline support", which the board holds as two epics, `SYNC` (Offline sync) and `CACHE` (Local cache), across the versions v1.0.0 and v1.1.0. Sources: `agboard epic show SYNC`, `agboard epic show CACHE`, `agboard show <id>` for #3 to #7, and the generated reports `docs/release/v1.0.0.md` and `docs/release/v1.1.0.md`. Every line below names the card, result or history entry it comes from. Nothing here is an achievement rate: the board holds four completion conditions for the theme, and each is judged on its own evidence.

## Epic SYNC: Offline sync

| Completion condition (from `epic show SYNC`) | Status | Evidence |
| --- | --- | --- |
| Edits made offline reach the server after reconnect without loss. | Advanced and verified | #3 `Queue edits while offline`: AC-1 done; its `## Verification` says 200 queued edits were replayed after a 5 minute outage with none lost; final result 1 links to the v1.0.0 release page and states `Published: v1.0.0`. |
| Conflicting edits are resolved without losing either side. | Advanced, not met | #4 `Merge conflicting edits on reconnect`: AC-1 (two-sided edits merge) done, AC-2 (no edit is silently dropped) not done; its `## Verification` says overlapping edits still pick one side. It has no result. History: planned for v1.0.0, then moved to v1.1.0 (`Changed planned version from v1.0.0 to v1.1.0.`). |

## Epic CACHE: Local cache

| Completion condition (from `epic show CACHE`) | Status | Evidence |
| --- | --- | --- |
| The last opened board opens with no network. | Verified; publication unconfirmed | #5 `Open the last board without network` (archived): AC-1 done; `## Verification` says a browser test opened the cached board with the network off. Its final result 1 says `Published: v1.0.0` but has no link, so the v1.0.0 report does not count it as published. |
| The cache stays under the configured size limit. | Not started | #6 `Limit the cache size` (v1.1.0): AC-1 not done; `## Verification` reads "Not started"; no result. |

Outside the four conditions, #7 `Show an offline banner` (CACHE, v1.1.0) is verified (AC-1 done, checked on desktop and phone width) and has a linked publication record for v1.1.0 (result 1). No completion condition of either epic names it, so it counts toward neither.

## What advanced, by version

- **v1.0.0** (report `docs/release/v1.0.0.md`): two planned cards, both verified (#3, #5), one card moved out (#4). One of the two verified cards has a linked publication record (#3).
- **v1.1.0** (report `docs/release/v1.1.0.md`): three planned cards. #7 is verified and has a linked publication record. #4 and #6 remain. #4 arrived from v1.0.0.

## Unconfirmed

- Whether #5 was published in v1.0.0. Result 1 of #5 claims it without a link. Ask for the release page or a package link, or record a linked result; until then it stays unconfirmed.
- Whether overlapping edits can ever lose data. #4 AC-2 is open and its own verification note says one side wins. The condition "without losing either side" is therefore not met.
- The size limit. #6 has no verification at all.
- Whether #7 has any effect on the theme's conditions. The epics do not mention it; a person would have to decide whether a condition is missing.

## Not claimed

- No percentage is given. Two cards and one condition are verified, one condition is partly met, one is untouched; these counts do not measure how close the theme is to finished, because the cards are not of equal size and the conditions are not a count of cards.
- No epic is closed. Both `EPIC.md` files keep every `- [ ]` unchecked until the people who own the conditions confirm them.

## Next

- #4 and #6, both planned for v1.1.0, are the work that decides whether the theme's conditions are met.
```
