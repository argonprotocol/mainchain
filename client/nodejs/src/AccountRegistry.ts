export class AccountRegistry {
  public namedAccounts: Map<string, string> = new Map();
  public me: string = 'me';

  constructor(name?: string) {
    if (name) {
      this.me = name;
    }
  }

  public getName(address: string): string | undefined {
    return this.namedAccounts.get(address);
  }

  public register(address: string, name: string): void {
    this.namedAccounts.set(address, name);
  }

  public static factory: (name?: string) => AccountRegistry = name => new AccountRegistry(name);
}
