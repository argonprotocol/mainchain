# Running a Miner in the Testnet

In this guide, we will walk through the steps to run a miner on the Argon test network.

You'll learn how to:

1. Setup a miner machine
2. Acquire Argons and Ownership Tokens
3. Bid for a Mining Slot
4. Start mining and watch for rewards

## 1. Setup a Miner Machine

### Minimum Server Requirements

Operating System: Ubuntu 22.04

CPU: 2x vCPU

Memory: 4 GB

Storage: 25 GB (possibly more if you run bitcoin on the same machine)

### Node Setup

You need the following software installed/accessible. There is
an [Ansible playbook](https://github.com/argonprotocol/argon-ansible) available to help with this setup. For this guide,
we'll show some of the high-level steps.

1. An NTP client to keep your system clock in sync with the network. You can install NTP with the following commands:
   ```bash
   sudo apt-get install ntpsec
   sudo ufw allow 123/udp
   sudo nano /etc/ntp.conf
   ```
   Replace the `server` lines with:
   ```
    server 0.pool.ntp.org iburst
    server 1.pool.ntp.org iburst
    server 2.pool.ntp.org iburst
    server 3.pool.ntp.org iburst
    ```
2. A Bitcoin node connected to the custom Argon Signet that supports Compact Block Filters. You can reference
   the [Bitcoin Core installation guide](https://bitcoin.org/en/full-node#linux-instructions) for Ubuntu.

   > You can install this on the same machine, but do note it will take up a few GB of storage. If you install it on
   another machine, modify your bitcoin.conf as appropriate.

   Your bitcoin.conf must include the following configs:
     ```bash
     chain=signet
     blockfilterindex=1
     server=1
     [signet]
     rpcauth={{ bitcoin_rpcauth }}
     rpcport=18332
     rpcbind=127.0.0.1
     rpcallowip=127.0.0.1/0
     signetchallenge=0014df6edb04cb5d8de5b15a63bdf5883f2eb6678b88
     signetseednode=bitcoin-node0.testnet.argonprotocol.org:38333
     ```
     ---
   NOTE: this does not exclude other configs you may need to run your bitcoin node. We are pruning by default in our own
   [testnet setup](https://github.com/argonprotocol/argon-ansible/tree/main/roles/bitcoin/templates/bitcoin.conf.j2).
3. The Argon software. You can find the latest release on the [releases page](
   https://github.com/argonprotocol/mainchain/releases/latest). You're looking for a file
   named `argon-node-v<VERSION>-x86_64-unknown-linux-gnu.tar.gz`. Download it to your server. You probably want to
   set
   this up as a systemd service on your own server. The ansible playbook will do this for you.

   You can also use the docker image published on
   the [GitHub Container Registry](https://github.com/argonprotocol/mainchain/pkgs/container/argon-miner).

   **Network (libp2p) Identity File**
   You'll need to create an identity file for your node. Internally, Argon uses [libp2p](https://libp2p.io) to
   discover and
   connect to the decentralized network. You can generate a libp2p identity with the following command:
      ```bash
       ./argon-node key generate-node-key --file /home/argon/argon-node.key
     ```

   **Start Script**
   You need to launch your node with configurations to connect to the Argon Testnet.
   ```bash
   ./argon-node --validator \
      --name "Your Node Name" \
      # Control the data location for your node
      # --base-path /path/to/your/node/data \
      # or a path to your testnet chain spec
      --chain testnet \
      # the rpc url for your signet bitcoin node with blockfilters enabled
      --bitcoin-rpc-url="http://bitcoin:<ENCODED_PASS>@127.0.0.1:38332" \
      # allow rpc on your local host only by default
      --rpc-port 9944 \
      # don't connect to local peers
      --no-mdns \
      # add detailed logs
      --detailed-log-output \
      # your node identity file for connecting to the network
      --node-key-file /home/argon/argon-node.key
   ```
   **Session Keys:**
   Once your node is up (the first time ONLY), you need to create session keys for your node. You can do this
   with
   the following command:
   ```bash
   curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "
   author_rotateKeys"}' http://localhost:9944/
      ```

## 2. Acquire Argons and Ownership Tokens

Mining requires you to have two tokens: Argons and Ownership Tokens. There are 10,000 mining slots available in Argon,
each lasting 10 days. So every day, you are bidding for 1 of 1,000 available slots. Bidding will continue until a random
block less than or equal to 200 blocks before the next slot begins (slots start every 1440 blocks).

At any given time, a mining slot requires you to own and lock 1/10,000th of the total Ownership Tokens in circulation.
And you can (optionally) put yourself ahead of someone else on the list by bidding more Argons than they have. You will
get these Argons back at the end of the slot (or if you lose your bid). Argons rented for this process must come from
a [Vault](./running-a-vault#mining-bonds).

You need to set up an account and acquire Argons to bid for a mining slot. You can do this by
following the steps in the [Argon Faucet Guide](./account-setup.md).

You'll also need to acquire Ownership Tokens. Once Argon is live, you will buy these off of decentralized like Uniswap,
or earn them during the first 10 days of mining (this time before Bidding begins is referred to as Slot Zero). In the
Testnet you can request Argons using a Discord bot just like
the [Argon Faucet](./account-setup.md#requesting-testnet-funds), but you'll use the slash following command instead:

```

bash
/ drip - ownership [address]

```

## 3. Bid for a Mining Slot

Now that you have an account with Argons and Ownership Tokens, you can bid for a mining slot. You're bidding for a 10-
day period starting at the next block that is divisible by 1440 blocks (eg, every 1440 blocks from the genesis block).
Mining bids close in a randomly chosen block within 200 blocks of the next slot. Mining bids normally begin after a 10
day bootstrap period called "Slot Zero". However, in the Testnet, you can start bidding right away.

There are 1,000 mining positions available every "slot", and 10,000 total miners. Each slot will last 10 days, so at any
given time, there are 10 overlapping cohorts of miners. Each day, 1,000 will rotate out and 1,000 will rotate in.

You are bidding for a slot, and can be outbid at any time by someone who "locks" more Argons than you. You can monitor
if you currently have a winning slot by looking at
the [Chain State](https://polkadot.js.org/apps/?rpc=wss://rpc.testnet.argonprotocol.org#/chainstate)
under **miningSlot** -> **nextSlotCohort**. If you are in the nextSlotCohort, you have a winning bid. You can also
monitor for events in
each block matching `SlotBidderReplaced` to see if you have been outbid. (Events can be monitored programmatically using
the `@argonprotocol/mainchain` node.js library, or using the `argon-client` rust library, which is
a [`subxt`](https://github.com/paritytech/subxt) based rust library).

To submit your bid, you'll need to submit the signing keys you'll use for the slot. These are the keys you generated
when
you created your node [here](#node-setup) -> Session Keys. You'll need to split the output of the `author_rotateKeys`
call into the two 32 byte keys (they'll in the same order
as the output, or there's a runtime
call [available](https://polkadot.js.org/apps/?rpc=wss://rpc.testnet.argonprotocol.org#/runtime)
called `sessionKeys` -> `decodeSessionKey`). If you want to more carefully create and backup your keys, you can also
generate them individually as shown [here](https://docs.substrate.io/tutorials/build-a-blockchain/add-trusted-nodes/).
You'll need to insert two Ed25519 keys with key-types of `gran` and one of `seal`.

You can bid for a slot by using the Polkadot.js
interface [here](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.testnet.argonprotocol.org#/extrinsics/decode/0x050001010000006400000000000000000000000000000000).
If you toggle to "Submission", you can submit your bid.
![Polkadot.js - Submit a bit](images/pjs-miningbid.png)

> NOTE: you'll want to review the Vaults and the terms they are offering for renting the Argons you want to bid with.
> That's available at Developer -> Chain State -> Vaults -> Vaults.
> ![Polkadot.js - Vaults](images/pjs-vaults.png)

### 4. Start Mining and Watch for Rewards

Once you have successfully bid for a mining slot, you can start mining. You will win blocks with an average equal split
with however many other active miners there are. A miner wins blocks in two ways:

1. Your node is selected as the XOR closest node to a block vote submitted in a notebook for the current tick. The miner
   with the closest XOR distance of their Authority ID (the key you registered as a *Session Key*) to the block vote key
   will win the block. This block will always take priority over the second method.
2. Your node solves a Proof of Compute (RandomX) hash that is less than the current difficulty target. These blocks are
   considered "secondary" and will only be included if no primary block is available. You can fill in as many "compute"
   blocks as you want, but you will only get rewards if you are able to include new Notebooks in the block.

You can monitor your mining rewards by checking
the [Block Explorer](https://polkadot.js.org/apps/?rpc=wss://rpc.testnet.argonprotocol.org#/explorer) and watching for
blocks created by your account.
![Polkadot.js - Block Explorer](images/pjs-blockexplorer.png)

Your rewards will consist of Argons and Ownership Tokens. Rewards start at 5 Argons and 5 Ownership Tokens per block,
and will decrease by half ever 2.1 million blocks (blocks are on average every minute). You can view your accumulated
Ownership tokens using
the [Chainstate](https://polkadot.js.org/apps/?rpc=wss://rpc.testnet.argonprotocol.org#/chainstate) tab in Polkadot.js
and looking up `Ownership -> Accounts (your account)`.
![Polkadot.js - Ownership](images/pjs-ownership.png)
