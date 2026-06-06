import { parseArgs as parseNodeArgs } from 'node:util';

export function parseArgs(argv: string[]) {
  const parsed: Record<string, string> = {};
  const { tokens } = parseNodeArgs({
    args: argv,
    options: {},
    allowPositionals: true,
    strict: false,
    tokens: true,
  });

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token.kind !== 'option') continue;

    const next = tokens[index + 1];
    if (next?.kind === 'positional' && next.index === token.index + 1) {
      parsed[token.name] = next.value;
      index += 1;
      continue;
    }

    parsed[token.name] = 'true';
  }

  return parsed;
}

export function getRequiredArg(rawArgs: Record<string, string>, key: string) {
  const value = getOptionalArg(rawArgs, key);
  if (!value) {
    throw new Error(`Missing required --${key}`);
  }

  return value;
}

export function getOptionalArg(rawArgs: Record<string, string>, key: string) {
  return rawArgs[key]?.trim();
}

export function getOptionalBigInt(rawArgs: Record<string, string>, key: string) {
  const value = getOptionalArg(rawArgs, key);
  if (!value) {
    return undefined;
  }

  return BigInt(value);
}

export function stringifyJson(value: unknown) {
  return JSON.stringify(
    value,
    (_key, entry: unknown): unknown => (typeof entry === 'bigint' ? entry.toString() : entry),
    2,
  );
}
