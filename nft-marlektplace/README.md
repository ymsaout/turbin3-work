# NFT Marketplace — Metaplex Core

Anchor program implementing a fully on-chain NFT marketplace using the Metaplex Core standard.  
Supports SOL payments, SPL token payments, and a counter-offer escrow system.

## Stack

| Dependency | Version |
|---|---|
| anchor-lang | 0.32.1 |
| anchor-spl | 0.32.1 |
| mpl-core | 0.11.2 |

## Architecture

Two state accounts plus one per-offer:

```
seeds:
  Marketplace       → ["marketplace",        name_bytes]
  Treasury          → ["treasury",           marketplace_key]   — vault SOL fees
  Rewards mint      → ["rewards_mint",       marketplace_key]   — SPL reward token
  Listing           → ["listing",            marketplace_key, asset_key]
  Offer             → ["offer",              asset_key, buyer_key]
  Collection auth   → ["collection_authority", collection_key]  — signing PDA
```

NFTs are transferred **custodially** to the listing PDA at list time and transferred back on buy/delist/accept-offer. The listing PDA signs CPI transfers via `invoke_signed`.

### Marketplace account

```rust
pub struct Marketplace {
    pub admin: Pubkey,
    pub fee: u16,          // basis points (e.g. 250 = 2.5 %)
    pub bump: u8,
    pub treasury_bump: u8,
    pub rewards_bump: u8,
    pub name: String,      // max 32 chars — part of PDA seed
}
```

### Listing account

```rust
pub struct Listing {
    pub maker: Pubkey,
    pub asset: Pubkey,
    pub price: u64,
    pub payment_mint: Option<Pubkey>,  // None = SOL, Some = SPL token
    pub bump: u8,
}
```

### Offer account

```rust
pub struct Offer {
    pub buyer: Pubkey,
    pub asset: Pubkey,
    pub amount: u64,  // SOL lamports escrowed in the PDA
    pub bump: u8,
}
```

## Instructions

### Core (transcript)

| Instruction | Description |
|---|---|
| `initialize` | Creates Marketplace PDA, SOL treasury vault, SPL rewards mint |
| `list` | Locks NFT in listing PDA, stores price and optional payment mint |
| `buy` | SOL payment: maker receives `price − fee`, treasury receives `fee`, taker receives NFT + reward token |

### Homework

| Instruction | Description |
|---|---|
| `delist` | Returns NFT to maker, closes listing PDA |

### Challenges

| Instruction | Description |
|---|---|
| `buy_with_token` | SPL token payment via `transfer_checked`: split between `maker_payment_ata` and `treasury_ata` |
| `make_offer` | Escrows SOL into Offer PDA, requires active SOL listing for that asset |
| `accept_offer` | Maker accepts: lamports transferred from offer PDA → maker + treasury, NFT → buyer, reward minted |
| `cancel_offer` | Buyer cancels: Offer PDA closed, all lamports (escrow + rent) returned to buyer |

## Payment flows

**SOL listing (`payment_mint = None`):**
```
taker ──(price − fee)──▶ maker
taker ──(fee)──────────▶ treasury (SystemAccount PDA)
listing PDA ──(NFT)───▶ taker
marketplace ──(1 token)▶ taker rewards ATA
```

**SPL token listing (`payment_mint = Some(mint)`):**
```
taker_payment_ata ──(price − fee)──▶ maker_payment_ata   (transfer_checked)
taker_payment_ata ──(fee)──────────▶ treasury_ata         (transfer_checked)
listing PDA ──(NFT)─────────────────▶ taker
marketplace ──(1 token)─────────────▶ taker rewards ATA
```

**Counter-offer flow:**
```
make_offer:   buyer ──(amount)──▶ offer PDA (escrow)
accept_offer: offer PDA ──(amount − fee)──▶ maker
              offer PDA ──(fee)──────────▶ treasury
              listing PDA ──(NFT)────────▶ buyer
              rent of offer ──────────────▶ buyer (Anchor close)
cancel_offer: offer PDA ──(amount + rent)──▶ buyer (Anchor close)
```

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
  nft-marketplace
  createCollection: 38dh87DfFWduSyr2Ayrj5LXvWTNCuexno2f8y1APDvVc69E3G9UwJeV11bBy4rczDb5hRQj1PBHaWqRSvQ5zoBmk
    ✔ creates a collection (50ms)
  marketplace already exists, skipping init
    ✔ initializes the marketplace
  list (SOL): 3wxVyjXyNdVf5UKvhXQeHAwZBZ9c7J7B6sHRPS7k1Qg2J8z4AFjWbQTvBGPv7WYN9ViP5ti9sjpJpdMP3M4RLeij
  buy (SOL): 4ZipfZGSQfoQbsFJKE9ZwXkwcaE51ceE5cs1JnFvszmqmPswmY4kgk2er3M5GfXBuumPwHwUhMKHxQkLWTmQCnTa
    ✔ lists an NFT and buys it with SOL (119ms)
  delist: 2kaqNFBEKCBKvwDJFBXvMYKFRSYxgRZ7GZs4TfTy9f1h7dCFNuVUNw6mUrJ5taQ7ybFnsWqFTuydGFJfgoD7ctxD
    ✔ lists and delists an NFT (116ms)
  buy_with_token: 2c9nXqrJJd385QYfV6zqTjTiLVUSk3aJEkGh2kGGdtcuaERU48B72W4vz67hxeaLFwC5honv7Nci4LxS9fsyEbE5
    ✔ lists and buys with SPL token (688ms)
  accept_offer: 56F32HefftWyoSutDcgm6qSmt4zHHYSYBkG7PLA2f3t1A46YN8TyxXfTwQVAvuDkC515zwBAbbbieayHCETinf1W
    ✔ makes an offer and the maker accepts it (189ms)
  cancel_offer: 3bGBBBYVxfLDKkQ7KnzYCHtXmEbVy3oJKe6WemDFLc1N5EFh6mR7endCHX1B9LEtf7BuBtpttszrLfoXaM1wJZcR
    ✔ makes an offer and the buyer cancels it (162ms)

  7 passing (2s)
```

## Key design decisions

**Custodial listing** — the NFT is transferred to the listing PDA on `list`, not frozen. Ownership is unambiguous on-chain and no additional freeze plugin is needed.

**`payment_mint: Option<Pubkey>` on Listing** — one listing struct supports both SOL and any SPL token. `buy` enforces `is_none()`, `buy_with_token` enforces `== Some(payment_mint)`.

**SOL offer escrow via lamport manipulation** — the Offer PDA holds the escrowed SOL. On `accept_offer`, `try_borrow_mut_lamports()` is used to transfer lamports out of the program-owned PDA without a system_program CPI (which would fail for non-system-owned accounts). Anchor's `close = buyer` then returns the rent.

**anchor 0.32 + mpl-core 0.11** — mpl-core account types (`BaseAssetV1`, `BaseCollectionV1`) cannot be used with `Account<'info, T>` due to a trait mismatch between anchor 0.31 (mpl-core's dependency) and 0.32. Collection and asset are declared as `UncheckedAccount`. `AccountInfo` temporaries in `TransferV1CpiBuilder` calls are stored in local variables to avoid borrow lifetime errors.
