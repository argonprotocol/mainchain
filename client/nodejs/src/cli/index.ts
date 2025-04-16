#!/bin/env node
import { addGlobalArgs, applyEnv, buildCli } from './setup';

const program = buildCli();
addGlobalArgs(program);
// load env
applyEnv(program);
program.parseAsync(process.argv).catch(console.error);
