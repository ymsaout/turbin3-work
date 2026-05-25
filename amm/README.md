# AMM — Constant-Product Market Maker on Solana

An on-chain Automated Market Maker (AMM) program built with [Anchor](https://www.anchor-lang.com/) on Solana. It implements the classic **constant-product formula** (`x · y = k`) used by Uniswap v2, and relies on Dean Little's [`constant-product-curve`](https://github.com/deanmlittle/constant-product-curve) crate for all curve mathematics.

---

## Table of Contents

- [Theory](#theory)
- [Architecture](#architecture)
- [Accounts & PDAs](#accounts--pdas)
- [Instructions](#instructions)
- [Error Codes](#error-codes)
- [Project Structure](#project-structure)
- [Getting Started](#getting-started)
- [Running Tests](#running-tests)
- [Key Design Decisions](#key-design-decisions)

---

## Theory

### Constant-Product Curve

The pool holds two token reserves `x` and `y`. Their product is kept constant at `k`:

```
x · y = k
```

When a user swaps `Δx` tokens of X into the pool, the new Y reserve satisfies:

```
(x + Δx) · y₂ = k   →   y₂ = k / (x + Δx)
```

The user receives `Δy = y - y₂`. Because the curve is a hyperbola, the exchange rate worsens (price impact) as the trade size grows relative to the pool reserves.

**Example** (from the Turbin3 lecture):

| State | X reserve | Y reserve | k |
|---|---|---|---|
| Initial | 20 | 30 | 600 |
| After swap 5 X | 25 | 24 | 600 |
| After swap 5 X again | 30 | 20 | 600 |

For the first swap: `5 X → 6 Y`. For the second: `5 X → 4 Y`. The increasing X reserve reduces X's relative value, so each subsequent unit of X buys fewer Y tokens.

### Liquidity Providers

Liquidity providers (LPs) deposit both tokens and receive **LP tokens** proportional to their share of the pool. LP tokens are burned on withdrawal to reclaim the underlying assets plus any accumulated fees.

### Fee Mechanism

A swap fee (expressed in basis points, e.g. `100` = 1 %) is deducted from each trade. The fee stays in the pool and accrues to all LPs proportionally.

### Impermanent Loss

When pool prices diverge from external markets, LPs may end up holding a different token ratio than what they deposited. The resulting shortfall compared to simply holding the tokens is called **impermanent loss**. Trading fees are the compensation that makes providing liquidity profitable despite this risk.

### Arbitrage

Price divergences between pools are closed by arbitrageurs who buy cheaply in one market and sell at a higher price in another. This benefits the ecosystem: arbitrageurs profit and prices converge across venues.

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                       AMM Program                        │
│                                                          │
│  initialize ──► Config PDA  ──► mint_lp PDA             │
│                              ──► vault_x ATA (config)    │
│                              ──► vault_y ATA (config)    │
│                                                          │
│  deposit ──► CPI: transfer X/Y from user → vault        │
│           ──► CPI: mint_to LP → user_lp ATA             │
│                                                          │
│  withdraw ──► CPI: transfer X/Y from vault → user       │
│            ──► CPI: burn LP from user_lp ATA            │
│                                                          │
│  swap ──► init ConstantProductCurve (xy=k)              │
│        ──► CPI: transfer in from user → vault            │
│        ──► CPI: transfer out from vault → user           │
└──────────────────────────────────────────────────────────┘
```

---

## Accounts & PDAs

### Config

The pool state, one per `(mint_x, mint_y, seed)` triplet.

| Field | Type | Description |
|---|---|---|
| `seed` | `u64` | Allows multiple pools for the same token pair |
| `authority` | `Option<Pubkey>` | Optional admin; set to `None` for permissionless pools |
| `mint_x` | `Pubkey` | Token X mint |
| `mint_y` | `Pubkey` | Token Y mint |
| `fee` | `u16` | Swap fee in basis points (0–10 000) |
| `locked` | `bool` | Emergency lock flag; blocks deposits, withdrawals, and swaps |
| `config_bump` | `u8` | Saved PDA bump (avoids re-deriving) |
| `lp_bump` | `u8` | Saved LP mint PDA bump |

**Seeds:** `["config", mint_x, mint_y, seed_le_bytes]`

### Mint LP

An SPL Token mint whose authority is the `Config` PDA. Minted on deposit, burned on withdrawal.

**Seeds:** `["lp", config]`

### Vault X / Vault Y

Associated Token Accounts (ATAs) owned by the `Config` PDA. They hold the pool's token reserves.

- `vault_x` = ATA(`config`, `mint_x`)
- `vault_y` = ATA(`config`, `mint_y`)

---

## Instructions

### `initialize(seed, fee, authority)`

Creates the pool config, the LP mint, and both vault ATAs in a single transaction.

| Argument | Type | Description |
|---|---|---|
| `seed` | `u64` | Pool discriminator (enables multiple pools per token pair) |
| `fee` | `u16` | Swap fee in basis points |
| `authority` | `Option<Pubkey>` | Admin pubkey, or `None` for permissionless |

**Accounts:**

| Account | Writable | Signer | Description |
|---|---|---|---|
| `initializer` | ✓ | ✓ | Pays for account rent |
| `mint_x` | | | Token X mint |
| `mint_y` | | | Token Y mint |
| `mint_lp` | ✓ | | LP mint PDA (init) |
| `vault_x` | ✓ | | Vault ATA for X (init) |
| `vault_y` | ✓ | | Vault ATA for Y (init) |
| `config` | ✓ | | Pool config PDA (init) |
| `token_program` | | | SPL Token |
| `system_program` | | | System |
| `associated_token_program` | | | ATA |

---

### `deposit(amount, max_x, max_y)`

Deposits tokens X and Y into the pool and mints LP tokens to the caller.

| Argument | Type | Description |
|---|---|---|
| `amount` | `u64` | Number of LP tokens to mint |
| `max_x` | `u64` | Maximum X tokens willing to deposit (slippage guard) |
| `max_y` | `u64` | Maximum Y tokens willing to deposit (slippage guard) |

**Behavior:**
- If the pool is **empty** (first deposit): the caller sets the initial ratio freely. `max_x` and `max_y` become the deposited amounts.
- If the pool **already has liquidity**: `xy_deposit_amounts_from_l` computes the required X and Y proportional to the current reserves.

**Accounts:**

| Account | Writable | Signer | Description |
|---|---|---|---|
| `user` | ✓ | ✓ | Liquidity provider |
| `mint_x` | | | |
| `mint_y` | | | |
| `config` | | | Pool config PDA |
| `mint_lp` | ✓ | | LP mint (to mint into) |
| `vault_x` | ✓ | | Pool reserve for X |
| `vault_y` | ✓ | | Pool reserve for Y |
| `user_x` | ✓ | | User's token X ATA |
| `user_y` | ✓ | | User's token Y ATA |
| `user_lp` | ✓ | | User's LP ATA (init_if_needed) |
| `token_program` | | | |
| `system_program` | | | |
| `associated_token_program` | | | |

---

### `withdraw(amount, min_x, min_y)`

Burns LP tokens and returns a proportional share of both reserves to the caller.

| Argument | Type | Description |
|---|---|---|
| `amount` | `u64` | Number of LP tokens to burn |
| `min_x` | `u64` | Minimum X tokens expected back (slippage guard) |
| `min_y` | `u64` | Minimum Y tokens expected back (slippage guard) |

**Accounts:** same structure as `deposit` (direction of transfers is reversed).

---

### `swap(is_x, amount, minimum)`

Swaps one token for the other using the constant-product formula, with a fee deducted before the curve calculation.

| Argument | Type | Description |
|---|---|---|
| `is_x` | `bool` | `true` = deposit X, receive Y · `false` = deposit Y, receive X |
| `amount` | `u64` | Amount to deposit into the pool |
| `minimum` | `u64` | Minimum amount to receive (slippage guard) |

**Accounts:**

| Account | Writable | Signer | Description |
|---|---|---|---|
| `user` | ✓ | ✓ | Trader |
| `mint_x` | | | |
| `mint_y` | | | |
| `config` | | | Pool config PDA |
| `mint_lp` | | | Read LP supply to initialise curve |
| `vault_x` | ✓ | | |
| `vault_y` | ✓ | | |
| `user_x` | ✓ | | init_if_needed |
| `user_y` | ✓ | | init_if_needed |
| `token_program` | | | |
| `system_program` | | | |
| `associated_token_program` | | | |

---

## Error Codes

| Code | Message | Trigger |
|---|---|---|
| `PoolLocked` | Pool is locked | `config.locked == true` |
| `InvalidAmount` | Invalid amount: must be greater than zero | `amount == 0` or curve returns 0 |
| `SlippageExceeded` | Slippage limit exceeded | Curve output below the caller's minimum |
| `InsufficientTokenX` | Insufficient token X | Computed X deposit > `max_x`, or X received < `min_x` |
| `InsufficientTokenY` | Insufficient token Y | Computed Y deposit > `max_y`, or Y received < `min_y` |

`CurveError` variants from the `constant-product-curve` crate are mapped to `AmmError` via a `From` implementation.

---

## Project Structure

```
amm/
├── Anchor.toml                          Anchor workspace config
├── Cargo.toml                           Workspace manifest
├── rust-toolchain.toml                  Pinned Rust toolchain (1.89.0)
├── .gitignore
└── programs/
    └── amm/
        ├── Cargo.toml                   Crate manifest & dependencies
        └── src/
            ├── lib.rs                   Program entry point — declares 4 instructions
            ├── errors.rs                AmmError enum + From<CurveError>
            ├── state/
            │   ├── mod.rs
            │   └── config.rs            Config account struct (118 bytes)
            └── instructions/
                ├── mod.rs
                ├── initialize.rs        Init config, LP mint, and vault ATAs
                ├── deposit.rs           Deposit X/Y, mint LP tokens
                ├── withdraw.rs          Burn LP tokens, withdraw X/Y
                └── swap.rs              Constant-product swap with fee
        └── tests/
            └── amm_tests.rs             LiteSVM integration tests (native Rust)
```

### Key Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `anchor-lang` | 0.32.1 | Solana program framework |
| `anchor-spl` | 0.32.1 | SPL Token CPI helpers |
| `constant-product-curve` | git | AMM curve math (Dean Little) |
| `litesvm` | 0.4 | Lightweight Solana VM for testing |

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (toolchain pinned to 1.89.0 via `rust-toolchain.toml`)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) ≥ 2.0
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) 0.32.x
- A local keypair at `~/.config/solana/id.json`

### Build

```bash
cd amm

# Compile the on-chain program to a BPF .so binary
anchor build

# Sync the generated program ID into declare_id! and Anchor.toml
anchor keys sync
```

After `anchor keys sync`, update the `PROGRAM_ID` constant in [programs/amm/tests/amm_tests.rs](programs/amm/tests/amm_tests.rs) to match.

### Deploy (localnet)

```bash
# Terminal 1 — start a local validator
solana-test-validator

# Terminal 2 — deploy
anchor deploy
```

---

## Running Tests

Tests use **LiteSVM**, a lightweight in-process Solana VM written in Rust. They run natively (no BPF compilation needed for the test binary itself) and are significantly faster than TypeScript + Bankrun or a live validator.

```bash
# Build the program first (generates the .so loaded by the tests)
anchor build
anchor keys sync   # update PROGRAM_ID in tests/amm_tests.rs

# Run all integration tests
cargo test

# Run a specific test
cargo test test_swap -- --nocapture
```

### Test Coverage

| Test | What it verifies |
|---|---|
| `test_initialize` | Config, LP mint, and both vault ATAs exist after `initialize` |
| `test_deposit` | User LP ATA is created and non-empty after the first deposit |
| `test_withdraw` | Half the LP position can be burned without error |
| `test_swap` | A 1 000 X → Y swap succeeds with a 1:1 seeded pool |

> **Tip:** After each test run, add assertions on account data (e.g. parse the `Config` struct, check vault balances) to catch regressions in state.

---

## Key Design Decisions

### PDA Seeds

The `Config` PDA incorporates both mint addresses and a user-supplied `seed`:

```
["config", mint_x_bytes, mint_y_bytes, seed_le_bytes]
```

This allows multiple independent pools for the same token pair (e.g. different fee tiers).

### Precision

`ConstantProduct::init` is called with `precision = None`, which defaults to `1_000_000` (equivalent to 6 decimal places). The static helpers `xy_deposit_amounts_from_l` and `xy_withdraw_amounts_from_l` also receive `1_000_000` for consistency. This minimises rounding error in the integer arithmetic.

### Bump Caching

Both `config_bump` and `lp_bump` are stored in the `Config` account. Subsequent instructions read the stored bump directly and call `create_program_address` (O(1)) instead of the more expensive `find_program_address` (iterative search).

### Signer Seeds Pattern

When the `Config` PDA must sign (minting LP tokens, withdrawing from vaults), the full seed slice including the bump is constructed as local variables to satisfy Rust's borrow checker:

```rust
let seed_bytes = self.config.seed.to_le_bytes();
let mint_x_key = self.mint_x.key();
let mint_y_key = self.mint_y.key();
let seeds = [
    b"config".as_ref(),
    mint_x_key.as_ref(),
    mint_y_key.as_ref(),
    seed_bytes.as_ref(),
    &[self.config.config_bump],
];
let signer_seeds = &[&seeds[..]];
```

### Emergency Lock

Setting `config.locked = true` halts all user-facing instructions (`deposit`, `withdraw`, `swap`) without affecting the on-chain state, giving the authority time to investigate an issue before re-enabling the pool.

### `init-if-needed`

`user_lp` (on deposit) and `user_x` / `user_y` (on swap) use `init_if_needed` so that a user's first interaction with the pool does not require a separate ATA creation transaction. The `init-if-needed` Anchor feature is therefore required in `Cargo.toml`.
