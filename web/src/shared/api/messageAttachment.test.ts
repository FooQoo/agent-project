import { describe, expect, it } from 'vitest';
import {
  messageAttachmentUrl,
  resolveHistoryImage,
  resolveMessageImage,
} from './messageAttachment';

const project = 'a'.repeat(32);

describe('messageAttachmentUrl', () => {
  it('builds the route of a message image', () => {
    expect(messageAttachmentUrl(project, 'Q-3', 'screen.png')).toBe(
      `/api/projects/${project}/messages/Q-3/attachments/screen.png`,
    );
  });

  it('encodes the message ID and the name', () => {
    expect(messageAttachmentUrl(project, 'Q-3', 'a b/../c.png')).toBe(
      `/api/projects/${project}/messages/Q-3/attachments/a%20b%2F..%2Fc.png`,
    );
  });
});

describe('resolveMessageImage', () => {
  it('resolves attachments/<name> from a body', () => {
    expect(resolveMessageImage(project, 'Q-3', 'attachments/screen.png')).toBe(
      `/api/projects/${project}/messages/Q-3/attachments/screen.png`,
    );
  });

  it.each([
    `${'https:'}//e.test/a.png`,
    '/etc/passwd',
    '../attachments/a.png',
    'attachments/',
    'attachments/../QA.md',
    'attachments/sub/a.png',
    'screen.png',
  ])('leaves %s unresolved', (reference) => {
    expect(resolveMessageImage(project, 'Q-3', reference)).toBeNull();
  });
});

describe('resolveHistoryImage', () => {
  it('resolves attachments/<Q-n>/<name> from a history entry', () => {
    expect(resolveHistoryImage(project, 'attachments/Q-348/screen.png')).toBe(
      `/api/projects/${project}/messages/Q-348/attachments/screen.png`,
    );
  });

  it('keeps two questions with one image name apart', () => {
    expect(resolveHistoryImage(project, 'attachments/Q-1/a.png')).not.toBe(
      resolveHistoryImage(project, 'attachments/Q-2/a.png'),
    );
  });

  it.each([
    `${'https:'}//e.test/a.png`,
    'attachments/screen.png',
    'attachments/Q-1/',
    'attachments//a.png',
    'attachments/Q-0/a.png',
    'attachments/Q-01/a.png',
    'attachments/Q-x/a.png',
    'attachments/R-1/a.png',
    'attachments/Q-1/.hidden',
    'attachments/Q-1/a b.png',
    'attachments/Q-1/../a.png',
    'attachments/Q-1/sub/a.png',
    `attachments/Q-1/${'a'.repeat(101)}`,
  ])('leaves %s unresolved', (reference) => {
    expect(resolveHistoryImage(project, reference)).toBeNull();
  });
});
