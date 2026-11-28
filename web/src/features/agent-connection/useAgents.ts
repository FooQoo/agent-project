import { useCallback } from 'react';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadClient, useReadQuery } from '../../shared/data/react';
import { useWrite } from '../../shared/data/react/use-write';
import {
  type Agent,
  type AgentTool,
  type NewAgent,
  loadAgents,
  registerAgent,
  setAgentTool,
} from './api';

/**
 * The project's agents, with the two writes the settings dialog makes: register an agent and
 * change an agent's tool. A finished write reads the list again.
 */
export function useAgents(projectId: string | null) {
  const client = useReadClient();
  const key = queryKeys.agents(projectId ?? '');
  const query = useReadQuery<Agent[]>({
    queryKey: key,
    queryFn: () => loadAgents(projectId ?? ''),
    enabled: projectId !== null,
  });
  const refresh = useCallback(() => client.invalidateQueries({ queryKey: key }), [client, key]);
  const register = useWrite<Agent, NewAgent>({
    mutationKey: ['agent-register', projectId],
    mutationFn: (input) => registerAgent(projectId ?? '', input),
    onSettled: refresh,
  });
  const changeTool = useWrite<Agent, { agentId: string; tool: AgentTool }>({
    mutationKey: ['agent-tool', projectId],
    mutationFn: ({ agentId, tool }) => setAgentTool(projectId ?? '', agentId, tool),
    onSettled: refresh,
  });
  return { query, register, changeTool };
}
