# New task template

Start the `TASK.md` of a new task from this template. The frontmatter is input: Agent Project generates the first history entry.

```markdown
---
acceptance:
  - id: AC-1
    text: (Describe a condition that can be verified.)
    done: false
# What the task will deliver and when. Write what each result is and the condition for
# delivering it, not the work steps. A task with no intermediate result lists only the final one.
resultPlans:
  - content: (What the requester can look at, such as "The working endpoint".)
    condition: (When to deliver it, such as "When the endpoint answers its first request".)
  - content: (The finished outcome.)
    condition: (When to deliver it, such as "When every acceptance condition is verified".)
    final: true
# Creation input: Agent Project generates the initial history entry.
# Stored entries have exactly at (UTC YYYY-MM-DDTHH:MM:SSZ) and reason (content).
# at uses whole seconds only; timezone offsets and fractional seconds are rejected.
history: []
---

## Background

(Why this task exists and what it builds on.)

## What to do

(The change to make, in a few sentences or a short list.)

## Approach

(How you will do it, and any decision that shapes it.)

## Verification results

(Add concise verified acceptance results and artifact references here after verification.)
```

`resultPlans` is optional. Each plan has `content` (what is delivered) and `condition` (when it should be delivered); mark the final one with `final: true` (at most one). Leave out `id`: the store gives each plan its `RP-<n>` in the same save. If no intermediate result is needed, write only the final plan, or no plans at all. Do not use plans for work steps or a delivery order. Later, `result add <id> --plan RP-<n>` delivers one (`agpj skills results`).

The body headings above are recommended, not required: a body without them is still valid. Write the headings in the language of the board, per the language rule of the skill. Record history only in the history frontmatter, as decisions and grounds; do not add an execution-history section to the body.
