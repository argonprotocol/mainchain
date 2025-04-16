import { Command, Option } from '@commander-js/extra-typings';
import accountCli from './accountCli';
import { configDotenv } from 'dotenv';
import Path from 'node:path';
import vaultCli from './vaultCli';
import miningCli from './miningCli';
import liquidityCli from './liquidityCli';
import bitcoinCli from './bitcoinCli';
import { parseSubaccountRange } from '../Accountset';

export function globalOptions(program: Command) {
  return program.optsWithGlobals() as IGlobalOptions;
}

export function buildCli() {
  return new Command('Argon CLI')
    .option('-e, --env <path>', 'The path to the account .env file to load')
    .addOption(
      new Option('-u, --mainchain-url <url>', 'The mainchain URL to connect to')
        .default('wss://rpc.argon.network')
        .env('MAINCHAIN_URL'),
    )
    .addOption(
      new Option(
        '--account-file-path <jsonPath>',
        'The path to your json seed file from polkadotjs',
      ).env('ACCOUNT_JSON_PATH'),
    )
    .addOption(
      new Option(
        '--account-suri <secretUri>',
        'A secret uri (suri) to use for the account',
      ).env('ACCOUNT_SURI'),
    )
    .addOption(
      new Option(
        '--account-passphrase <password>',
        'The password for your seed file',
      ).env('ACCOUNT_PASSPHRASE'),
    )
    .addOption(
      new Option(
        '-s, --subaccounts <range>',
        'Restrict this operation to a subset of the subaccounts (eg, 0-10)',
      )
        .env('SUBACCOUNT_RANGE')
        .argParser(parseSubaccountRange),
    )
    .addCommand(accountCli())
    .addCommand(vaultCli())
    .addCommand(miningCli())
    .addCommand(liquidityCli())
    .addCommand(bitcoinCli());
}

export type IGlobalOptions = ReturnType<ReturnType<typeof buildCli>['opts']>;

export function addGlobalArgs(program: ReturnType<typeof buildCli>) {
  for (const command of program.commands) {
    command.configureHelp({ showGlobalOptions: true });
    for (const nested of command.commands) {
      nested.configureHelp({ showGlobalOptions: true });
    }
  }
}

export function applyEnv(
  program: ReturnType<typeof buildCli>,
): string | undefined {
  program.parseOptions(process.argv);
  const { env } = program.optsWithGlobals();
  if (env) {
    const envPath = Path.resolve(process.cwd(), env);
    const res = configDotenv({ path: envPath });
    if (res.parsed?.ACCOUNT_JSON_PATH) {
      // ensure path is relative to the env file if provided that way
      process.env.ACCOUNT_JSON_PATH = Path.relative(
        envPath,
        process.env.ACCOUNT_JSON_PATH!,
      );
    }
  }
  return env;
}
