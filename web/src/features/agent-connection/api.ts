import { i18n } from '../../shared/i18n';
import { getJson, projectPath, authHeaders } from '../../shared/api/http';
import { postTask } from '../../shared/api/write';
import { writeFailure } from '../../shared/api/writeErrors';
import type { ApiError } from '../../shared/api/contracts';

/** The AI tools an agent can run its tasks with, spelled as the server stores them. */
export const AGENT_TOOLS = ['codex', 'claude-code'] as const;
export type AgentTool = (typeof AGENT_TOOLS)[number];

export interface Agent {
  id: string;
  name: string;
  role: string;
  /** The tool that runs this agent's tasks; `null` while none is chosen. */
  tool: AgentTool | null;
  isDefault: boolean;
  archivedAt: string | null;
}

export interface ConnectCode {
  code: string;
  expiresAt: string;
}

export interface NewAgent {
  name: string;
  role: string;
  tool: AgentTool | null;
}

const agentsPath = (projectId: string) => `${projectPath(projectId)}/agents`;

/** The project's live agents, oldest first. */
export async function loadAgents(projectId: string): Promise<Agent[]> {
  const { agents } = await getJson<{ agents: Agent[] }>(agentsPath(projectId));
  return agents.filter((agent) => agent.archivedAt === null);
}

export function defaultAgentId(agents: Agent[]): string | null {
  return agents.find((agent) => agent.isDefault)?.id ?? null;
}

export async function registerAgent(projectId: string, input: NewAgent): Promise<Agent> {
  const body: Record<string, string> = { name: input.name, role: input.role };
  if (input.tool) body.tool = input.tool;
  const { agent } = await postTask<{ agent: Agent }>(agentsPath(projectId), body);
  return agent;
}

/** Changes only the tool of an agent; its name and role stay. */
export async function setAgentTool(
  projectId: string,
  agentId: string,
  tool: AgentTool,
): Promise<Agent> {
  const response = await fetch(`${agentsPath(projectId)}/${encodeURIComponent(agentId)}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
      'X-Agpj-Language': i18n.resolvedLanguage === 'ja' ? 'ja' : 'en',
    },
    body: JSON.stringify({ tool }),
  });
  const body = (await response.json()) as { agent?: Agent; error?: ApiError['error'] };
  if (!response.ok || body.agent === undefined) {
    throw writeFailure(
      body.error?.reason ?? `Request failed (${response.status})`,
      response.status,
      body.error?.code ?? null,
      body.error?.message ?? null,
    );
  }
  return body.agent;
}

export async function issueConnectCode(projectId: string, agentId: string): Promise<ConnectCode> {
  const { connectCode } = await postTask<{ connectCode: ConnectCode }>(
    `${agentsPath(projectId)}/${encodeURIComponent(agentId)}/connect-code`,
    {},
  );
  return connectCode;
}
