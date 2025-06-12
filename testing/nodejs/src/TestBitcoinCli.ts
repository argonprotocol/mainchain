import * as child_process from 'node:child_process';
import { projectRoot } from './index';
import * as Path from 'node:path';

export default class TestBitcoinCli {
  /**
   * Returns the localhost address of the notary (NOTE: not accessible from containers)
   */
  public static run(command: string): string {
    const binPath = Path.join(`${projectRoot()}`, 'target/debug/argon-bitcoin-cli');

    try {
      return child_process
        .execSync(`${binPath} ${command}`, {
          encoding: 'utf8',
        })
        .trim();
    } catch (e) {
      console.error(`Error running command: ${command}`);
      console.error((e as any).stdout);
      throw e;
    }
  }
}
