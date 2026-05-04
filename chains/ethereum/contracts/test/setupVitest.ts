import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const contractsRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));

process.env.HARDHAT_CONFIG = resolve(contractsRoot, 'hardhat.config.ts');
process.chdir(contractsRoot);
