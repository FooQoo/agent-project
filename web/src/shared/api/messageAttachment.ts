import type { ProjectId } from './contracts';
import { projectPath } from './http';

const BODY_PREFIX = 'attachments/';

/** The URL of an image sent with a message (`GET …/messages/{id}/attachments/{name}`). */
export function messageAttachmentUrl(
  projectId: ProjectId,
  messageId: string,
  name: string,
): string {
  return `${projectPath(projectId)}/messages/${encodeURIComponent(messageId)}/attachments/${encodeURIComponent(name)}`;
}

/**
 * The URL for an image reference in a message body, which is written `attachments/<name>`.
 * Returns `null` for anything else (an external URL, a nested or relative path), so the caller
 * can leave it unresolved.
 */
export function resolveMessageImage(
  projectId: ProjectId,
  messageId: string,
  reference: string,
): string | null {
  if (!reference.startsWith(BODY_PREFIX)) return null;
  const name = reference.slice(BODY_PREFIX.length);
  if (name === '' || name.includes('/') || name.includes('\\')) return null;
  return messageAttachmentUrl(projectId, messageId, name);
}

const QUESTION_ID = /^Q-[1-9][0-9]*$/;
// As the server: 1 to 100 of letters, digits, `.`, `_`, `-`, and no leading dot.
const PLAIN_NAME = /^(?!\.)[A-Za-z0-9._-]{1,100}$/;

/**
 * The URL for an image reference in a history entry, which is written
 * `attachments/<Q-n>/<name>`: the server names the question when it copies a question body into
 * a task's history, because the history has no attachments folder of its own. The question ID
 * and the file name are checked as the server checks them; anything else (an external URL, a
 * reference that names no question, a nested path) returns `null`.
 */
export function resolveHistoryImage(projectId: ProjectId, reference: string): string | null {
  if (!reference.startsWith(BODY_PREFIX)) return null;
  const parts = reference.slice(BODY_PREFIX.length).split('/');
  if (parts.length !== 2) return null;
  const [messageId, name] = parts;
  if (messageId === undefined || name === undefined) return null;
  if (!QUESTION_ID.test(messageId) || !PLAIN_NAME.test(name)) return null;
  return messageAttachmentUrl(projectId, messageId, name);
}
