# RAP setup, migration, and Tailscale transition

This guide explains how to set up Remote Agent Project (RAP) with Codex and Claude Code, migrate from a legacy Agent Project file store, and switch remote access from ngrok to Tailscale.

## Prerequisites and naming

- Install `codex` (0.172.0 or newer) and/or `claude` (Claude Code 2.1.x or newer) and sign in locally.
- The `rap` subcommand/binary name was checked locally (`which rap` reported no conflicting command). Packaging continues under the `agent-project` distribution until the final CLI naming decision in the release gate.

## Remote access with Tailscale (retiring ngrok)

RAP standardizes mobile remote access on Tailscale.

1. Connect both your host machine and your phone to the same tailnet.
2. Run `agpj setup remote --provider tailscale` to switch the saved configuration to Tailscale.
3. Verify access from your phone and confirm that revoked or expired device sessions are rejected.
4. If an older `.internal/monitor-connection.json` still contains an ngrok domain or keychain token, switching the provider to `tailscale` (or disabling remote access) ensures no ngrok tunnel starts automatically. Remove the unused ngrok token from the OS keychain after verifying Tailscale access.

## Migrating legacy Agent Project data

Before migrating a legacy `FileStore` (`shared/`):

1. Stop all legacy resident sessions and `agpj serve`. RAP refuses migration while any legacy session is active.
2. Create a verified backup copy of `shared/` with SHA-256 digests (`copy_and_digest_tree`) so you can roll back cleanly.
3. Run the migration into `rap.db`. Migration preserves:
   - Epics, tasks, and parent-epic membership
   - Task history (`legacy_history`)
   - Questions and answers
   - Published results, attachments (content-addressed in `rap.db.objects`), and ID/hash mappings (`legacy_id_map`)
   - Existing Markdown (`EPIC.md`, `TASK.md`, `docs/release/*.md`, and project `docs/`) untouched on disk
4. The migrated RAP database always starts with `auto_run = false` and `max_sessions = 1`. Review the imported board before enabling automatic execution.

## Rolling back

If you need to return to the legacy store:

1. Stop RAP and confirm no agent attempts are running.
2. Restore the `shared/` directory from your backup copy and verify file digests (`digest_tree`).
3. Start the legacy `agpj serve` against the restored `shared/` directory.
