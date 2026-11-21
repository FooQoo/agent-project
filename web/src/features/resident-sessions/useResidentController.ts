import { useCallback, useEffect, useState } from 'react';
import type { ProjectId } from '../../shared/api/contracts';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadClient } from '../../shared/data/react';
import { useWrite } from '../../shared/data/react/use-write';
import {
  ResidentRequestError,
  getModels,
  launchSession,
  saveAutoStart,
  stopSession,
  type ModelOption,
  type SessionTool,
} from './api';
import { notifyResidentChanged } from './events';
import type { Pending, ResidentProblem } from './types';
import { autoStartIsOn, cliProblemOf, initialChoice, type Choice, type CliProblem } from './state';
import { DIALOG_POLL_MS, useResidentSession } from './useResidentSession';

interface Models {
  tool: SessionTool;
  options: ModelOption[];
  /** Why the list could not be read; only the default model is offered then. */
  reason: string | null;
  problem: ResidentProblem | null;
}

const problemOf = (cause: unknown): ResidentProblem =>
  cause instanceof ResidentRequestError
    ? { code: cause.code, reason: cause.reason, unreachable: cause.status === 0 }
    : { code: null, reason: null, unreachable: true };

/**
 * The state behind the dialog. It reads the sessions every two seconds while it is mounted, keeps
 * the tool and model the person chose (which apply from the next start), and runs Start, Stop,
 * Restart and the auto-start checkbox. A request in flight disables the controls.
 */
export function useResidentController(projectId: ProjectId) {
  const { snapshot, failed } = useResidentSession(projectId, DIALOG_POLL_MS);
  const [chosen, setChosen] = useState<Choice | null>(null);
  const [models, setModels] = useState<Models | null>(null);
  const [problem, setProblem] = useState<ResidentProblem | null>(null);
  const [launchProblem, setLaunchProblem] = useState<ResidentProblem | null>(null);

  const health = snapshot?.health ?? null;
  const choice: Choice | null = chosen ?? (health ? initialChoice(health) : null);
  const tool = choice?.tool ?? null;

  const loadModels = useCallback(async () => {
    if (tool === null) return;
    try {
      const list = await getModels(projectId, tool);
      setModels({ tool, options: list.models, reason: list.reason, problem: null });
    } catch (cause) {
      setModels({ tool, options: [], reason: null, problem: problemOf(cause) });
    }
  }, [projectId, tool]);

  useEffect(() => {
    void loadModels();
  }, [loadModels]);

  // What follows a write: the sessions are read again at once (a read that is running is joined),
  // and the footer and the entry row are told. The write stays pending until the read is back.
  const client = useReadClient();
  const changed = useCallback(async () => {
    await client.invalidateQueries(
      { queryKey: queryKeys.residentSessions(projectId), exact: true },
      { cancelRefetch: false },
    );
    notifyResidentChanged();
  }, [client, projectId]);

  const launch = useWrite<unknown, { tool: SessionTool; model: string }>({
    mutationKey: ['resident-launch', projectId],
    mutationFn: ({ tool: next, model }) => launchSession(projectId, next, model),
    onError: (cause) => setLaunchProblem(problemOf(cause)),
    onSettled: changed,
  });
  const stopping = useWrite<unknown>({
    mutationKey: ['resident-stop', projectId],
    mutationFn: () => stopSession(projectId),
    onError: (cause) => setProblem(problemOf(cause)),
    onSettled: changed,
  });
  const autoStartWrite = useWrite<unknown, Parameters<typeof saveAutoStart>[1]>({
    mutationKey: ['resident-autostart', projectId],
    mutationFn: (input) => saveAutoStart(projectId, input),
    onSuccess: changed,
    onError: (cause) => setProblem(problemOf(cause)),
  });
  const pending: Pending = launch.isPending ? 'starting' : stopping.isPending ? 'stopping' : null;
  const savingAuto = autoStartWrite.isPending;

  const autoStart = health ? autoStartIsOn(health.autoStart, projectId) : false;

  /** Saves the auto-start choice when it is on, so it keeps following the chosen tool and model. */
  const followAutoStart = (next: Choice) => {
    setProblem(null);
    autoStartWrite.mutate({ enabled: true, ...next });
  };

  const choose = (next: Choice) => {
    setChosen(next);
    setLaunchProblem(null);
    if (autoStart) followAutoStart(next);
  };

  const start = () => {
    if (!choice || launch.isWriting() || stopping.isWriting()) return;
    setChosen(choice);
    setLaunchProblem(null);
    setProblem(null);
    launch.mutate({ tool: choice.tool, model: choice.model });
  };

  const stop = () => {
    if (launch.isWriting() || stopping.isWriting()) return;
    // The choice stays what it was after the session is gone.
    if (choice) setChosen(choice);
    setProblem(null);
    stopping.mutate();
  };

  const setAutoStart = (enabled: boolean) => {
    if (!choice) return;
    if (enabled) return followAutoStart(choice);
    setProblem(null);
    autoStartWrite.mutate({ enabled: false });
  };

  const checkAgain = async () => {
    setLaunchProblem(null);
    await loadModels();
  };

  // The CLI problem of the chosen tool: the model list says "not installed" before any launch, and a
  // launch says "not logged in".
  const cliProblem: CliProblem | null =
    cliProblemOf(launchProblem?.code ?? null) ??
    (models?.tool === tool ? cliProblemOf(models.problem?.code ?? null) : null);
  const cliReason =
    (launchProblem && cliProblemOf(launchProblem.code) ? launchProblem.reason : null) ??
    (models?.tool === tool ? (models.problem?.reason ?? null) : null);

  return {
    snapshot,
    failed,
    choice,
    models: models?.tool === tool ? models : null,
    pending,
    savingAuto,
    autoStart,
    cliProblem,
    cliReason,
    /** A failed launch that is not a CLI problem. */
    launchProblem: launchProblem && !cliProblemOf(launchProblem.code) ? launchProblem : null,
    problem,
    chooseTool: (next: SessionTool) => choose({ tool: next, model: '' }),
    chooseModel: (model: string) => choice && choose({ ...choice, model }),
    start,
    stop,
    setAutoStart,
    checkAgain: () => void checkAgain(),
  };
}
