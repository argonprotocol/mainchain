use crate::CryptoScheme::Sr25519;
use crate::test_utils::*;
use crate::*;
use argon_primitives::{BalanceChange, LocalchainAccountId};
use polkadot_sdk::*;
use sp_keyring::Ed25519Keyring::Ferdie;
use sp_keyring::Sr25519Keyring::{Alice, Bob};

#[allow(dead_code)]
struct TestNetwork {
  alice: Localchain,
  bob: Localchain,
  notary: MockNotary,
  notary_clients: NotaryClients,
}

impl TestNetwork {
  async fn new(bob_pool: SqlitePool) -> anyhow::Result<Self> {
    let alice_pool = create_pool().await?;
    let notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&notary, Ferdie).await?;
    let bob = mock_localchain(&bob_pool, &Bob.to_seed(), Sr25519, &notary_clients).await;
    let alice = mock_localchain(&alice_pool, &Alice.to_seed(), Sr25519, &notary_clients).await;
    Ok(TestNetwork {
      alice,
      bob,
      notary,
      notary_clients,
    })
  }

  async fn add_mainchain_funds(&self, localchain: &Localchain, amount: u128) -> anyhow::Result<()> {
    self
      .notary
      .create_claim_from_mainchain(
        localchain.begin_change(),
        amount,
        AccountStore::parse_address(&localchain.address().await?)?,
      )
      .await?;
    Ok(())
  }

  async fn register_balance_tip(
    &self,
    localchain: &Localchain,
    balance_change: &BalanceChange,
  ) -> anyhow::Result<()> {
    let account_id = balance_change.account_id.clone();
    let address = AccountStore::to_address(&account_id);
    let account = localchain
      .accounts()
      .get(address, balance_change.account_type, 1)
      .await?;
    let balance_change_row = localchain
      .balance_changes()
      .get_latest_for_account(account.id)
      .await?
      .expect("no balance change");
    let balance_tip = balance_change_row.get_balance_tip(&account)?;
    let mut state = self.notary.state.lock().await;
    state.balance_tips.insert(
      LocalchainAccountId::new(account.get_account_id32()?, account.account_type),
      argon_notary_apis::localchain::BalanceTipResult {
        tick: localchain.ticker.current(),
        balance_tip: balance_tip.tip().into(),
        notebook_number: balance_change
          .previous_balance_proof
          .as_ref()
          .unwrap()
          .notebook_number,
      },
    );
    Ok(())
  }
}

#[sqlx::test]
async fn test_will_consolidate_accounts(pool: SqlitePool) -> anyhow::Result<()> {
  let network = TestNetwork::new(pool).await?;
  network.add_mainchain_funds(&network.alice, 5_000).await?;

  let _ = network
    .alice
    .transactions()
    .send(3500, Some(vec![network.bob.address().await?]))
    .await?;

  let _ = network.alice.transactions().send(1000, None).await?;

  let mut jump_accounts = 0;
  let alice_accounts = network.alice.accounts().list(Some(true)).await?;
  for account in alice_accounts {
    let balance = network
      .alice
      .balance_changes()
      .get_latest_for_account(account.id)
      .await?;
    if balance.is_some() && balance.unwrap().balance != "0" && account.hd_path.is_some() {
      println!(
        "need to consolidate account {:?} {:?}",
        account.account_type, account.address
      );
      jump_accounts += 1;
    }
  }

  let result = network.alice.balance_sync().sync(None).await?;
  assert_eq!(result.jump_account_consolidations.len(), jump_accounts);

  Ok(())
}

#[sqlx::test]
async fn test_will_handle_broken_consolidate_accounts(pool: SqlitePool) -> anyhow::Result<()> {
  let network = TestNetwork::new(pool.clone()).await?;
  network
    .add_mainchain_funds(&network.alice, 5_000_000)
    .await?;

  let _1 = network.alice.transactions().send(500_000, None).await?;
  let _2 = network.alice.transactions().send(500_000, None).await?;
  let _3 = network.alice.transactions().send(500_000, None).await?;

  let mut did_break_account = false;
  let alice_accounts = network.alice.accounts().list(Some(true)).await?;
  for account in alice_accounts {
    let Some(balance) = network
      .alice
      .balance_changes()
      .get_latest_for_account(account.id)
      .await?
    else {
      continue;
    };
    if balance.balance != "0" && account.hd_path.is_some() && !did_break_account {
      did_break_account = true;
      network
        .notary
        .set_bad_tip_error(
          account.get_account_id32()?,
          account.account_type,
          argon_notary_apis::Error::BalanceTipMismatch {
            provided_tip: None,
            stored_tip: None,
            change_index: 0,
          },
        )
        .await;
    }
  }

  let result = network.alice.balance_sync().sync(None).await?;
  assert_eq!(result.jump_account_consolidations.len(), 2);

  Ok(())
}

#[sqlx::test]
async fn test_will_sync_client_channel_holds(pool: SqlitePool) -> anyhow::Result<()> {
  let network = TestNetwork::new(pool.clone()).await?;
  network.notary.ticker.lock().await.tick_duration_millis = 200;
  network.bob.ticker.ticker.write().tick_duration_millis = 200;
  network.alice.ticker.ticker.write().tick_duration_millis = 200;

  network
    .add_mainchain_funds(&network.alice, 5_000_000)
    .await?;

  let channel_hold = network
    .alice
    .transactions()
    .create_channel_hold(500_000, network.bob.address().await?, None, None, None)
    .await?;
  let channel_expiration = channel_hold.channel_hold().await.expiration_tick;
  let json = channel_hold.export_for_send().await?;

  network
    .register_balance_tip(
      &network.alice,
      &channel_hold
        .channel_hold()
        .await
        .get_initial_balance_change(),
    )
    .await?;

  network
    .bob
    .open_channel_holds()
    .import_channel_hold(json)
    .await?;

  {
    let result = network.alice.balance_sync().sync(None).await?;
    assert_eq!(result.jump_account_consolidations.len(), 1);
    assert_eq!(result.channel_hold_notarizations.len(), 0);
  }

  let pending_tips = network.notary.get_pending_tips().await;
  network.notary.create_notebook_header(pending_tips).await;
  // update start time to be after the channel hold expiration
  let sleep_time = {
    let ticker = network.notary.ticker.lock().await;
    let next_tick_time = ticker.time_for_tick(channel_expiration as u64 + 1);

    next_tick_time.saturating_sub(ticker.now_adjusted_to_ntp())
  };
  tokio::time::sleep(Duration::from_millis(sleep_time)).await;
  {
    let result = network.bob.balance_sync().sync(None).await?;
    assert_eq!(result.jump_account_consolidations.len(), 0);
    assert_eq!(result.channel_hold_notarizations.len(), 1);
    let hold = result.channel_hold_notarizations[0].channel_holds[0].clone();
    assert_eq!(hold.settled_amount(), 5_000);
  }
  let pending_tips = network.notary.get_pending_tips().await;
  assert_eq!(pending_tips.len(), 3);
  network.notary.create_notebook_header(pending_tips).await;
  {
    let result = network.alice.balance_sync().sync(None).await?;
    assert_eq!(result.jump_account_consolidations.len(), 1);
    assert_eq!(result.channel_holds_updated().len(), 1);
    let hold = result.channel_holds_updated[0].clone();
    assert_eq!(hold.settled_amount(), 5_000);
  }

  Ok(())
}
