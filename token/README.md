# Ykevred Token (YKE)

SPL Token created on Solana Devnet.

## Token Information

| Field | Value |
|-------|-------|
| **Name** | Ykevred |
| **Symbol** | YKE |
| **Mint Address** | `53uQFaPsd5ADTUmXMahxW63TZttyLtyuwqJWqiQPHJv6` |
| **Decimals** | 6 |

## Transaction Hashes

| Action | Transaction Hash |
|--------|------------------|
| **Mint TX** | `46edSVVNskk7sJEXK6tchuPeMyEAG9pRPuxny9AULSPPQdcwy1txS8xXRBsGjQgAtvKTRranp6nzy22CkukLGToA` |
| **Transfer TX** | `JYR8a1JonS7zKYsCsuocKN26XKQsJwD9Gt6s7S3kuGeUrkZtb8tN9jbWtp47Cy2MZ2dqXJThYUcnKx3SAWR2JPb` |

## Links

- [View Token on Solana Explorer](https://explorer.solana.com/address/53uQFaPsd5ADTUmXMahxW63TZttyLtyuwqJWqiQPHJv6?cluster=devnet)
- [View Mint Transaction](https://explorer.solana.com/tx/46edSVVNskk7sJEXK6tchuPeMyEAG9pRPuxny9AULSPPQdcwy1txS8xXRBsGjQgAtvKTRranp6nzy22CkukLGToA?cluster=devnet)
- [Token Image on Arweave](https://gateway.irys.xyz/GsuL3SxzBNxzAZn9rfWbXf3SU33vKGp364aKVVwuNpun)
- [Token Metadata](https://gateway.irys.xyz/73YnWj6WYmNhAiyMyM3UUZJ96h3tggggYzmSQcr6qWYC)

## Token Image

![Ykevred Token](./token-image.png)

---

## How to Create Your Own SPL Token

### Prerequisites

1. Install dependencies:
```bash
cd ts
yarn install
```

2. Create a wallet file `ts/turbin3-wallet.json` with your Solana keypair (or symlink to `~/.config/solana/id.json`)

3. Make sure you have devnet SOL for transaction fees

### Step 1: Create the Mint

Edit `ts/cluster1/spl_init.ts` and run:
```bash
yarn spl_init
```
This creates the token mint address. **Save this address** for the next steps.

### Step 2: Upload Token Image

1. Add your image to `ts/cluster1/assets/`
2. Edit `ts/cluster1/nft_image.ts` with your image filename
3. Run:
```bash
yarn nft_image
```
**Save the image URI** returned.

### Step 3: Upload Metadata JSON

1. Edit `ts/cluster1/nft_metadata.ts`:
   - Set your token name, symbol, description
   - Use the image URI from Step 2
2. Run:
```bash
yarn nft_metadata
```
**Save the metadata URI** returned.

### Step 4: Link Metadata On-Chain

1. Edit `ts/cluster1/spl_metadata.ts`:
   - Set the mint address from Step 1
   - Set name, symbol, and metadata URI from Step 3
2. Run:
```bash
yarn spl_metadata
```

### Step 5: Mint Tokens

1. Edit `ts/cluster1/spl_mint.ts`:
   - Set the mint address from Step 1
   - Adjust the amount to mint
2. Run:
```bash
yarn spl_mint
```
**Save the mint transaction hash** for your submission.

### Step 6: Transfer Tokens

1. Edit `ts/cluster1/spl_transfer.ts`:
   - Set the mint address from Step 1
   - Set the recipient wallet address
   - Adjust the amount to transfer
2. Run:
```bash
yarn spl_transfer
```

### Summary of Commands

```bash
yarn spl_init        # 1. Create mint
yarn nft_image       # 2. Upload image
yarn nft_metadata    # 3. Upload metadata JSON
yarn spl_metadata    # 4. Link metadata on-chain
yarn spl_mint        # 5. Mint tokens
yarn spl_transfer    # 6. Transfer tokens
```
