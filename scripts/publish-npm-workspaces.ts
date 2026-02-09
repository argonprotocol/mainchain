import { execSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

type Workspace = {
  location: string;
  name?: string;
  version?: string;
  publishPriority: number;
};

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

let tag = 'dev';
let dryRun = false;
for (let i = 2; i < process.argv.length; i += 1) {
  const arg = process.argv[i];
  if (arg === '--tag') {
    const next = process.argv[i + 1];
    if (!next) fail('Missing value for --tag');
    tag = next;
    i += 1;
    continue;
  }
  if (arg === '--dry-run') {
    dryRun = true;
    continue;
  }
  fail(`Unknown argument: ${arg}`);
}

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..');

const workspaceLines = execSync('yarn workspaces list --json', { encoding: 'utf8', cwd: repoRoot })
  .trim()
  .split('\n')
  .filter(Boolean);

const workspaces: Workspace[] = workspaceLines
  .map((line) => JSON.parse(line) as { location: string })
  .map(({ location }) => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, location, 'package.json'), 'utf8')) as {
      name?: string;
      version?: string;
      private?: boolean;
    };

    return {
      location,
      name: pkg.name,
      version: pkg.version,
      publishPriority: location.startsWith('localchain/npm/') ? 0 : location === 'localchain' ? 2 : 1,
      private: Boolean(pkg.private),
    };
  })
  .filter((workspace) => !workspace.private)
  .sort((a, b) => a.publishPriority - b.publishPriority || a.location.localeCompare(b.location));

for (const workspace of workspaces) {
  if (!workspace.name) {
    fail(`Workspace at ${workspace.location} is missing "name"`);
  }
  if (!workspace.version) {
    fail(`Workspace ${workspace.name} at ${workspace.location} is missing "version"`);
  }

  if (dryRun) {
    console.log(
      `[dry-run] Would publish ${workspace.name}@${workspace.version} from ${workspace.location} with tag ${tag}`,
    );
    continue;
  }

  const alreadyPublished = spawnSync('npm', ['view', `${workspace.name}@${workspace.version}`, 'version'], {
    stdio: 'ignore',
    shell: false,
  }).status === 0;

  if (alreadyPublished) {
    console.log(`Skipping ${workspace.name}@${workspace.version}; already published`);
    continue;
  }

  console.log(`Publishing ${workspace.name}@${workspace.version} from ${workspace.location}`);
  const publishResult = spawnSync(
    'npm',
    ['publish', '--access', 'public', '--tag', tag, `--workspace=${workspace.location}`],
    {
      stdio: 'inherit',
      shell: false,
      cwd: repoRoot,
    },
  );
  if (publishResult.status !== 0) {
    fail(`Command failed: npm publish --workspace=${workspace.location}`);
  }
}
