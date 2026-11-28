import { afterEach, expect, it, vi } from 'vitest';
import { defaultAgentId, issueConnectCode, loadAgents, registerAgent, setAgentTool } from './api';

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

const agent = {
  id: 'A-2',
  name: 'Screens',
  role: 'UI',
  tool: 'codex',
  isDefault: false,
  archivedAt: null,
};

it('lists only live agents and finds the default one', async () => {
  const gone = { ...agent, id: 'A-3', archivedAt: '2026-10-10T00:00:00Z' };
  const fallback = { ...agent, id: 'A-1', isDefault: true, tool: null };
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue(Response.json({ agents: [fallback, agent, gone] })),
  );

  const agents = await loadAgents('project-a');
  expect(agents.map((a) => a.id)).toEqual(['A-1', 'A-2']);
  expect(defaultAgentId(agents)).toBe('A-1');
  expect(defaultAgentId([agent] as never)).toBeNull();
});

it('registers an agent with its tool, and leaves the tool out when none is chosen', async () => {
  const fetcher = vi.fn().mockImplementation(async () => Response.json({ agent }, { status: 201 }));
  vi.stubGlobal('fetch', fetcher);

  await expect(
    registerAgent('project-a', { name: 'Screens', role: 'UI', tool: 'codex' }),
  ).resolves.toEqual(agent);
  expect(JSON.parse(fetcher.mock.calls[0]?.[1].body)).toEqual({
    name: 'Screens',
    role: 'UI',
    tool: 'codex',
  });
  await registerAgent('project-a', { name: 'Screens', role: '', tool: null });
  expect(JSON.parse(fetcher.mock.calls[1]?.[1].body)).toEqual({ name: 'Screens', role: '' });
});

it('changes only the tool with a PATCH', async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(Response.json({ agent: { ...agent, tool: 'claude-code' } }));
  vi.stubGlobal('fetch', fetcher);

  await expect(setAgentTool('project-a', 'A-2', 'claude-code')).resolves.toMatchObject({
    tool: 'claude-code',
  });
  expect(fetcher).toHaveBeenCalledWith(
    '/api/projects/project-a/agents/A-2',
    expect.objectContaining({ method: 'PATCH', body: '{"tool":"claude-code"}' }),
  );
});

it('fails a tool change that the server refuses', async () => {
  vi.stubGlobal(
    'fetch',
    vi
      .fn()
      .mockResolvedValue(
        Response.json({ error: { code: 'invalid_value', reason: 'bad tool' } }, { status: 400 }),
      ),
  );
  await expect(setAgentTool('project-a', 'A-2', 'codex')).rejects.toThrow();
});
