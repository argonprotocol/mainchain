#!/usr/bin/env node
import { addGlobalArgs, applyEnv, buildCli } from './clis';

const program = buildCli();
addGlobalArgs(program);
// load env
applyEnv(program);
program.parseAsync(process.argv).catch(console.error);
