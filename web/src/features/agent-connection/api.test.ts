import { afterEach, expect, it, vi } from 'vitest';
import { issueConnectCode } from './api';

afterEach(() => vi.unstubAllGlobals());

it('issues a code with the JSON body required by the local endpoint', async () => {
  const connectCode = { code: 'ABCD2345', expiresAt: '2026-10-10T01:10:00Z' };
  const fetcher = vi.fn().mockResolvedValue(Response.json({ connectCode }));
  vi.stubGlobal('fetch', fetcher);

  await expect(issueConnectCode('project-a', 'A-1')).resolves.toEqual(connectCode);
  expect(fetcher).toHaveBeenCalledWith(
    '/api/projects/project-a/agents/A-1/connect-code',
    expect.objectContaining({
      method: 'POST',
      headers: expect.objectContaining({ 'Content-Type': 'application/json' }),
      body: '{}',
    }),
  );
});
