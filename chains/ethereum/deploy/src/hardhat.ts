import '@nomicfoundation/hardhat-ethers';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const srcRoot = resolve(fileURLToPath(new URL('.', import.meta.url)));
export const deployRoot = resolve(srcRoot, '..');
export const ethereumRoot = resolve(deployRoot, '..');
export const contractsRoot = resolve(ethereumRoot, 'contracts');
export const repoRoot = resolve(ethereumRoot, '..', '..');
