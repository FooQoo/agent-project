import { spawnSync } from 'node:child_process';

const fixtureConnections = new Map();

export function fixtureEnv(home, project) {
  return {
    ...process.env,
    AGPJ_HOME: home,
    TZ: 'UTC',
    AGPJ_CONNECTION_SECRET: fixtureConnections.get(home)?.get(project),
  };
}

/** Give a disposable project's CLI fixture a connection from the real registry. */
export function connectFixtureAgent(home, project, binary = process.env.AGPJ_E2E_PHONE_BACKEND) {
  let connections = fixtureConnections.get(home);
  if (!connections) {
    connections = new Map();
    fixtureConnections.set(home, connections);
  }
  if (connections.has(project)) return;
  if (!binary) throw new Error('The phone backend is required to connect a fixture agent');
  const result = spawnSync(binary, ['--ignored', '--exact', 'connect_fixture_agent', '--nocapture'], {
    cwd: home,
    env: { ...fixtureEnv(home), AGPJ_E2E_CONNECT_PROJECT: project },
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    throw new Error(`Connecting fixture agent for ${project} failed: ${result.error ?? result.stderr}`);
  }
  const secret = result.stdout.match(/^CONNECTION (\S+)$/m)?.[1];
  if (!secret) throw new Error(`Connecting fixture agent for ${project} did not return a connection`);
  connections.set(project, secret);
}

export function forgetFixtureConnections(home, project) {
  if (project === undefined) fixtureConnections.delete(home);
  else fixtureConnections.get(home)?.delete(project);
}
