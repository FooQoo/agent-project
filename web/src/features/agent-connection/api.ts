import { getJson, projectPath } from '../../shared/api/http';
import { postTask } from '../../shared/api/write';

interface Agent {
  id: string;
  isDefault: boolean;
  archivedAt: string | null;
}

export interface ConnectCode {
  code: string;
  expiresAt: string;
}

export async function loadDefaultAgentId(projectId: string): Promise<string | null> {
  const { agents } = await getJson<{ agents: Agent[] }>(`${projectPath(projectId)}/agents`);
  return agents.find((agent) => agent.isDefault && agent.archivedAt === null)?.id ?? null;
}

export async function issueConnectCode(projectId: string, agentId: string): Promise<ConnectCode> {
  const { connectCode } = await postTask<{ connectCode: ConnectCode }>(
    `${projectPath(projectId)}/agents/${encodeURIComponent(agentId)}/connect-code`,
    {},
  );
  return connectCode;
}
