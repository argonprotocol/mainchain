export default interface Env {
  ACCOUNT_SURI?: string;
  ACCOUNT_PASSPHRASE?: string;
  ACCOUNT_JSON_PATH?: string;
  SUBACCOUNT_RANGE?: string;
  MAINCHAIN_URL?: string;
  // A mnemonic seed to use for the runtime keys
  KEYS_MNEMONIC?: string;
  // A version number for the keys
  KEYS_VERSION?: string;
}
