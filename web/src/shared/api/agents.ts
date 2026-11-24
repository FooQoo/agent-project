import { getJson, projectPath } from './http';

export interface Agent {
  id: string;
  name: string;
  role: string;
  isDefault: boolean;
  archivedAt: string | null;
  connection: { id: string } | null;
}

export const agentsPath = (projectId: string) => `${projectPath(projectId)}/agents`;
export async function loadAgents(projectId: string): Promise<Agent[]> {
  const { agents } = await getJson<{ agents: Agent[] }>(agentsPath(projectId));
  return agents.filter((agent) => agent.archivedAt === null);
}
