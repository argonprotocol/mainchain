import * as process from 'node:process';
import HttpProxy from 'http-proxy';
import * as child_process from 'node:child_process';
import * as http from 'node:http';
import * as url from 'node:url';
import * as net from 'node:net';
import * as Path from 'node:path';

export interface ITeardownable {
  teardown(): Promise<void>;
}

const toTeardown: ITeardownable[] = [];

let proxy: HttpProxy | null = null;
let proxyServer: http.Server | null = null;

export const SKIP_E2E = process.env.SKIP_E2E === 'true' || process.env.SKIP_E2E === '1';

export async function getProxy() {
  if (!proxy) {
    proxy = HttpProxy.createProxyServer({
      changeOrigin: true,
      ws: true,
      autoRewrite: true,
    });
    proxy.on('error', () => null);
    proxyServer = http.createServer(function (req, res) {
      const queryData = url.parse(req.url!, true).query;
      if (!queryData.target) {
        res.writeHead(500, { 'Content-Type': 'text/plain' });
        res.end('Target parameter is required');
        return;
      }
      console.log('Proxying http request', queryData.target);
      proxy?.web(req, res, { target: queryData.target as string });
    });
    proxyServer.on('upgrade', function (req, clientSocket, head) {
      const queryData = url.parse(req.url!, true).query;
      const target = url.parse(queryData.target as string);
      proxy?.ws(req, clientSocket, head, {
        target: target.href,
        ws: true,
      });
      clientSocket.on('error', console.error);
    });
    await new Promise<void>(resolve => proxyServer!.listen(0, resolve));
    toTeardown.push({
      teardown: () =>
        new Promise<void>(resolve => {
          proxy?.close();
          proxyServer?.close(_ => null);
          proxy = null;
          proxyServer = null;
          resolve();
        }),
    });
  }
  const port = (proxyServer!.address() as net.AddressInfo).port;
  return `ws://host.docker.internal:${port}`;
}

export function projectRoot() {
  if (process.env.ARGON_PROJECT_ROOT) {
    return Path.join(process.env.ARGON_PROJECT_ROOT);
  }
  return Path.join(__dirname, `../../..`);
}

export async function runTestScript(relativePath: string): Promise<string> {
  const scriptPath = Path.resolve(projectRoot(), relativePath);
  return child_process.execSync(scriptPath, { encoding: 'utf8' }).trim();
}

export async function getDockerPortMapping(
  containerName: string,
  port: number,
): Promise<string | undefined> {
  return child_process
    .execSync(`docker port ${containerName} ${port}`, { encoding: 'utf8' })
    .trim()
    .split(':')
    .pop();
}

export async function teardown() {
  for (const t of toTeardown) {
    try {
      await t.teardown().catch(console.error);
    } catch {}
  }
  toTeardown.length = 0;
}

export function cleanHostForDocker(host: string, replacer = 'host.docker.internal'): string {
  if (process.env.ARGON_USE_DOCKER_BINS) {
    return host
      .replace('localhost', replacer)
      .replace('127.0.0.1', replacer)
      .replace('0.0.0.0', replacer);
  }
  return host;
}

export function addTeardown(teardownable: ITeardownable) {
  toTeardown.push(teardownable);
}

export function runOnTeardown(teardown: () => Promise<void>) {
  addTeardown({ teardown });
}

export function closeOnTeardown<T extends { close(): Promise<void> }>(closeable: T): T {
  addTeardown({ teardown: () => closeable.close() });
  return closeable;
}

export function disconnectOnTeardown<T extends { disconnect(): Promise<void> }>(closeable: T): T {
  addTeardown({ teardown: () => closeable.disconnect() });
  return closeable;
}
