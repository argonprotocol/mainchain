/// Max balance changes that can be in a single notarization
#[cfg_attr(feature = "napi", napi)]
pub const NOTARIZATION_MAX_BALANCE_CHANGES: u32 =
  argon_primitives::MAX_BALANCE_CHANGES_PER_NOTARIZATION;

/// Max domains that can be in a single notarization
#[cfg_attr(feature = "napi", napi)]
pub const NOTARIZATION_MAX_DOMAINS: u32 = argon_primitives::MAX_DOMAINS_PER_NOTARIZATION;
/// Max notarizations that can be in a single notarization
#[cfg_attr(feature = "napi", napi)]
pub const NOTARIZATION_MAX_BLOCK_VOTES: u32 = argon_primitives::MAX_BLOCK_VOTES_PER_NOTARIZATION;
/// Number of ticks past the expiration of a channel_hold that a recipient has to claim. After this point, sender can recoup the channel_holded funds
#[cfg_attr(feature = "napi", napi)]
pub const CHANNEL_HOLD_CLAWBACK_TICKS: u32 = argon_primitives::CHANNEL_HOLD_CLAWBACK_TICKS;

/// Minimum milligons that can be settled in a channel_hold
#[cfg_attr(feature = "napi", napi)]
pub const CHANNEL_HOLD_MINIMUM_SETTLEMENT: u128 = argon_primitives::MINIMUM_CHANNEL_HOLD_SETTLEMENT;

/// Max versions that can be in a datastore zone record
#[cfg_attr(feature = "napi", napi)]
pub const DATASTORE_MAX_VERSIONS: u32 = argon_primitives::MAX_DATASTORE_VERSIONS;

/// Minimum domain name length
#[cfg_attr(feature = "napi", napi)]
pub const DOMAIN_MIN_NAME_LENGTH: u32 = argon_primitives::MIN_DOMAIN_NAME_LENGTH as u32;

/// Cost to lease a domain for 1 year
#[cfg_attr(feature = "napi", napi)]
pub const DOMAIN_LEASE_COST: u128 = argon_primitives::DOMAIN_LEASE_COST;
