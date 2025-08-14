export interface ArgonClientConfig {
  debug?: boolean;
  keysVersion?: number;
  keySeedOrMnemonic?: string;
  subaccountRange?: string;
}

let config: ArgonClientConfig = {};

// Safe environment variable access that works in both Node.js and browser
function getEnvVar(key: string): string | undefined {
  if (typeof process !== 'undefined' && process.env) {
    return process.env[key];
  }
  return undefined;
}

export function setConfig(newConfig: ArgonClientConfig): void {
  config = { ...config, ...newConfig };
}

export function getConfig(): ArgonClientConfig {
  return {
    debug: config.debug ?? getEnvVar('DEBUG') === 'true',
    keysVersion:
      config.keysVersion ??
      (getEnvVar('KEYS_VERSION') ? parseInt(getEnvVar('KEYS_VERSION')!) : undefined),
    keySeedOrMnemonic: config.keySeedOrMnemonic ?? getEnvVar('KEYS_MNEMONIC'),
    subaccountRange: config.subaccountRange ?? getEnvVar('SUBACCOUNT_RANGE') ?? '0-9',
  };
}
