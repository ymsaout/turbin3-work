# Vault Program

A Solana Vault program built with the Anchor framework. This program allows users to create a personal vault (PDA), deposit SOL, withdraw SOL, and close the vault to reclaim all funds.

## Program Instructions

| Instruction    | Description                                                      |
|----------------|------------------------------------------------------------------|
| `initialize`   | Creates a vault state account and derives a vault PDA for the user |
| `deposit`      | Transfers SOL from the user to the vault PDA                     |
| `withdraw`     | Transfers SOL from the vault PDA back to the user                |
| `close`        | Withdraws all remaining SOL and closes the vault state account   |

## Architecture

- **VaultState PDA** — `seeds = ["state", user_pubkey]` — stores the bumps for both PDAs
- **Vault PDA** — `seeds = ["vault", vault_state_pubkey]` — system account that holds SOL

## Prerequisites

- [Rust](https://rustup.rs/)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (v2.x)
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) (v0.32.1)
- [Surfpool](https://docs.surfpool.run/) (optional, recommended)
- [Node.js](https://nodejs.org/) and Yarn

## Build & Test

```bash
# Install dependencies
yarn install

# Build the program
anchor build

# Run tests (with solana-test-validator)
anchor test
```

### With Surfpool

[Surfpool](https://surfpool.run/) is a drop-in replacement for `solana-test-validator` that loads Mainnet programs on the fly and runs tests significantly faster.

```bash
# Terminal 1: start Surfpool
surfpool start --legacy-anchor-compatibility --watch

# Terminal 2: run tests against Surfpool
anchor test --skip-local-validator
```

## Tests Passing

### With Surfpool

```
  vault
  Initialize tx: 2ZBytfmTLfrT899YigkZuSnX8x2N24CzVB3odiG1SjByxhEdEhHX45iChzxCMdoTZBEuLKWfor7NzH4EAaB17ase
  Vault state bump: 255
  Vault bump: 255
    ✔ Initialize (69ms)
  Deposit tx: 5Ei3qU9xeLysRCFrydYRuNRCLxiSGnczUyDtSqCAZ98e4Fs6HkrxHYLRXBDATwxhS5HAqqtPaW1EJtzh1ruEyxEJ
  Vault balance: 2 SOL
    ✔ Deposit (138ms)
  Withdraw tx: 4tQHb5j5oNitoUWdhxSZVSVhMVuspAarM3zJmvcZrV7w78HUv1qFM5nUvQt6vDAvdKUXPgoHWvYgy8AN5X1qziSy
  Vault balance after withdraw: 1 SOL
    ✔ Withdraw
  Close tx: 5aZJmyFbFWA9YDpkwwVqWYZhQ5vXe6BsfZDP1P11s4GFekkcytgdk7vCAg2apv5g9ff3teRpaqNvv7qwKtypH5rr
  User recovered: 1.00095548 SOL (minus tx fees)
  Vault balance after close: 0 lamports
  Vault state account closed: true
    ✔ Close

  4 passing (239ms)
```
