# NFT Staking — Metaplex Core (non-custodial)

Anchor program implementing non-custodial NFT staking using the Metaplex Core standard.  
The NFT stays in the owner's wallet at all times — the program freezes it via the `FreezeDelegate` plugin and tracks staking state on-chain via the `Attributes` plugin.

## Stack

| Dependency | Version |
|---|---|
| anchor-lang | 0.32.1 |
| anchor-spl | 0.32.1 |
| mpl-core | 0.11.2 |

## Architecture

A single state account (`Config`, PDA) is used for the entire protocol. Staking state is stored **directly on the asset** via Metaplex Core plugins — no per-user PDA needed.

```
seeds:
  Config          → ["config",           collection_key]
  Rewards mint    → ["rewards_mint",      collection_key]
  Update authority→ ["update_authority",  collection_key]   (signing only, never initialized)
```

### Config account

```rust
pub struct Config {
    pub rewards_basis_points: u16,  // reward rate
    pub freeze_period: u32,         // minimum staking duration (days)
    pub config_bump: u8,
    pub rewards_bump: u8,
}
```

### Plugins used on each asset

| Plugin | Type | Purpose |
|---|---|---|
| `Attributes` | Authority-managed | Stores `staked` (bool) and `staked_at` (unix timestamp) |
| `FreezeDelegate` | Owner-managed | Freezes the asset to block transfers |

## Instructions

| Instruction | Description |
|---|---|
| `initialize` | Creates the `Config` PDA and the SPL rewards mint |
| `create_collection` | Creates a Metaplex Core collection with the program's PDA as update authority |
| `mint_asset` | Mints a Core asset into the collection |
| `stake` | Adds/updates `Attributes` (`staked=true`, `staked_at=now`) + adds `FreezeDelegate` (frozen) |
| `unstake` | Verifies freeze period elapsed, resets attributes, unfreezes asset, mints SPL rewards |

## Running tests

Requires [Surfpool](https://github.com/surfpool/surfpool) (MPL Core pre-deployed).

```bash
# Terminal 1
surfpool start --legacy-anchor-compatibility --watch

# Terminal 2
anchor test --skip-local-validator
```

## Test output

```
  nft-staking
  createCollection: 4Fsnwv8sLGcZWRNpMLy2wa2QBB3bcF5fh8MNtFBocdJsemRvq8nV56xaGEtgoZZCZM5hpZUxQQHmU4pHMVaaeQiD
    ✔ creates a collection (152ms)
  initialize: 5WjbG7UXEmpuwxN9KHAdAy9ZPBTJSyqyjW4Ro1mwHQqWLJvNdLH7Ea1U6k1CbyrHyK2YRgwz3RaUXgY8kUYa5qJP
    ✔ initializes the config (45ms)
  mintAsset: 3L1h4XcrJHQYaFquznT9V3N73FAHBTYKzchirr3wzyb9hb5d2RHfJ2VvjFmwzhnqdRmygbs2MbdAfwt2HESjRzDD
    ✔ mints an asset into the collection (43ms)
  stake: 3cDRoF3YbT2do1Jv8VyH9JkQxbKRBtGs36wTx6BLbXBZNTQ8MaEdqv8gKLe28DfaTGzFnCHJ1eoVxFWLV7bP2Cmz
    ✔ stakes the asset (42ms)
  unstake: 4oGovoFS3VHbDqPrsj2fEwfPQWXnQv2okG1nvSc4dcUPqmUVUEDtgTdY1isfCbKfQTSqKEZ5kGg3HfuNs4M4B3N3
    ✔ unstakes the asset and claims rewards (46ms)

  5 passing (334ms)
```

## Key design decisions

**No per-staking PDA** — staking state lives on the asset itself via the `Attributes` plugin. This saves rent and compute, and leverages Core's native plugin system instead of adding program-side account management.

**Non-custodial** — the asset never leaves the owner's wallet. Transferability is blocked by the `FreezeDelegate` plugin whose `init_authority` is set to the program's update authority PDA, allowing the program to unfreeze on unstake without owner co-signature.

**anchor 0.32 + mpl-core 0.11** — `BaseAssetV1` and `BaseCollectionV1` are handled as `UncheckedAccount` with manual borsh deserialization via `deserialize_reader` (not `try_from_slice`) to tolerate trailing plugin bytes in the account data.
