import { postTask, writeJson } from '../../shared/api/write';
import { agentsPath, type Agent } from '../../shared/api/agents';

export interface ConnectCode {
  code: string;
  expiresAt: string;
}

const agentPath = (projectId: string, agentId: string) =>
  `${agentsPath(projectId)}/${encodeURIComponent(agentId)}`;

export async function saveAgent(
  projectId: string,
  agent: Pick<Agent, 'name' | 'role'>,
  id?: string,
): Promise<Agent> {
  const path = id ? agentPath(projectId, id) : agentsPath(projectId);
  const result = await writeJson<{ agent: Agent }>(id ? 'PATCH' : 'POST', path, agent);
  return result.agent;
}

export async function issueConnectCode(
  projectId: string,
  agentId: string,
  replace = false,
): Promise<ConnectCode> {
  const { connectCode } = await postTask<{ connectCode: ConnectCode }>(
    `${agentPath(projectId, agentId)}/connect-code`,
    replace ? { switch: true } : {},
  );
  return connectCode;
}

export async function prepareDefaultAgent(projectId: string): Promise<void> {
  await postTask(`${agentsPath(projectId)}/default`, {});
}
