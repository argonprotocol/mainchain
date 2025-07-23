#!/usr/bin/env node
import { addGlobalArgs, applyEnv, buildCli } from './clis';
import { waitForLoad } from './index';

const program = buildCli();
addGlobalArgs(program);
// load env
applyEnv(program);

(async function main() {
  await waitForLoad();
  await program.parseAsync(process.argv);
})().catch(console.error);
