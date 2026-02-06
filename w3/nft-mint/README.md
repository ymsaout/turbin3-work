# NFT Mint - Turbin3 W3

## Overview

Create, mint, and trade NFTs on Solana devnet using Metaplex UMI and Irys for storage.

## NFT Details

- **Name:** Kevred Scientist
- **Symbol:** KVRDSCI
- **Description:** An ermine dressed as a scientist - Turbin3 Q1 2026

## Screenshot

![Kevred Scientist NFT](./ts/cluster1/assets/generug.png)

## Transaction Hashes

| Step | Hash / URI |
|------|-----------|
| Image Upload (Irys) | `https://gateway.irys.xyz/7nUoK9js26UjMMy6MQZYRTNunFPEisCqjAZwUiXEyHy4` |
| Metadata Upload (Irys) | `https://gateway.irys.xyz/5WA4PBDK61pJSPvGFVUgBJPiaSi6TbedFeS676xKFYDr` |
| NFT Mint TX | `4mCqjPFErZ2mQy4ANLxC1aGDNPCmzhULUBaSq927iXdyGkidaSDgigi1BgwqWqc4biEpK7Xg7s4XLVWXEYo4dqeU` |
| Mint Address | `8cgf55BtX1EKjeXb1nmqtdLokSete2NYnyWLF1hFLYji` |
| NFT Trade TX | `4Bieh1UetEKHQB5ARniyNx3iqnG74RjZX6MX9xWwddFL7aNDRRsmqjV2k3NPgBxK2hwPWAkGMcoAvoRjJRzjWbgb` |

### Explorer Links (Solscan)

- Mint TX: https://solscan.io/tx/4mCqjPFErZ2mQy4ANLxC1aGDNPCmzhULUBaSq927iXdyGkidaSDgigi1BgwqWqc4biEpK7Xg7s4XLVWXEYo4dqeU?cluster=devnet
- Mint Address: https://solscan.io/token/8cgf55BtX1EKjeXb1nmqtdLokSete2NYnyWLF1hFLYji?cluster=devnet
- Trade TX: https://solscan.io/tx/4Bieh1UetEKHQB5ARniyNx3iqnG74RjZX6MX9xWwddFL7aNDRRsmqjV2k3NPgBxK2hwPWAkGMcoAvoRjJRzjWbgb?cluster=devnet

## How to Run

### Prerequisites

- Node.js >= 18
- Yarn
- A funded Solana devnet wallet (`turbin3-wallet.json` in `ts/`)

### Steps

```bash
cd ts

# Install dependencies
yarn install

# 1. Upload the NFT image to Arweave via Irys
yarn nft_image

# 2. Copy the image URI into nft_metadata.ts, then upload metadata
yarn nft_metadata

# 3. Copy the metadata URI into nft_mint.ts, then mint the NFT
yarn nft_mint
```

## Reflection

### Problems and Limitations of Trading NFTs via Discord/Manual Wallets

1. **Trust Issues:** There is no escrow or atomic swap mechanism when trading NFTs via Discord DMs. One party must send first, creating a risk of the other party not fulfilling their side of the trade. This requires trusting a stranger, which is fundamentally against the ethos of trustless blockchain transactions.

2. **No Price Discovery:** Negotiating in Discord channels is inefficient. There is no transparent marketplace where sellers can list prices and buyers can compare. This leads to information asymmetry and potentially unfair trades.

3. **Manual Process & Human Error:** Copy-pasting wallet addresses and manually sending NFTs is error-prone. A single typo in an address means permanent loss of the NFT. There is no confirmation flow or safety net.

4. **No Transaction History / Provenance UI:** While the blockchain records everything, there is no user-friendly interface to review the trade history or verify provenance during Discord-based negotiations.

### Proposed Solutions

1. **Atomic Swap Programs:** Use an on-chain escrow/swap program (like Tensor, Magic Eden, or a custom Anchor program) that holds both NFTs and only completes the swap when both parties have deposited. This eliminates the trust problem entirely.

2. **NFT Marketplaces with Offer Systems:** Platforms like Tensor or Magic Eden provide listing, bidding, and instant swap functionalities with built-in escrow, price discovery, and transaction safety. Using these removes the need for manual coordination.

3. **OTC Swap dApps:** Specialized peer-to-peer swap dApps (e.g., using Solana's Token Swap program or dedicated OTC platforms) can allow two parties to create a swap proposal on-chain, where each party deposits their NFT, and the swap executes atomically only when both sides are fulfilled.

---

## Reproduce the Mint Yourself

Step-by-step guide to create and mint your own NFT on Solana devnet.

### 1. Prerequisites

- **Node.js** >= 18
- **Yarn** (`npm install -g yarn`)
- **Solana CLI** installed ([guide](https://docs.solana.com/cli/install-solana-cli-tools))

### 2. Set Up the Wallet

```bash
# Generate a new wallet (or use an existing one)
solana-keygen new -o ~/.config/solana/id.json

# Switch to devnet
solana config set --url devnet

# Airdrop some SOL for fees
solana airdrop 2
```

Then create a symlink to your wallet inside the `ts/` folder:

```bash
cd ts/
ln -s ~/.config/solana/id.json turbin3-wallet.json
```

### 3. Install Dependencies

```bash
cd ts/
yarn install
```

### 4. Prepare Your Image

Place your PNG image in `ts/cluster1/assets/`. Then update the filename in `nft_image.ts`:

```typescript
// line 19 of nft_image.ts
const image = await readFile("./cluster1/assets/your_image.png");

// line 22
const genericFile = createGenericFile(image, "your_image.png", {
    contentType: "image/png"
});
```

### 5. Upload the Image

```bash
yarn nft_image
# Output: Your image URI: https://gateway.irys.xyz/...
```

Copy the displayed URI.

### 6. Upload the Metadata

Open `ts/cluster1/nft_metadata.ts` and update:

```typescript
// Paste the image URI obtained in the previous step
const image = "https://gateway.irys.xyz/YOUR_IMAGE_URI";

const metadata = {
    name: "Your NFT Name",
    symbol: "SYMB",
    description: "Your description",
    image: image,
    attributes: [
        { trait_type: "ExampleTrait", value: "ExampleValue" }
    ],
    // ...
};
```

Then run:

```bash
yarn nft_metadata
# Output: Your metadata URI: https://gateway.irys.xyz/...
```

Copy the displayed URI.

### 7. Mint the NFT

Open `ts/cluster1/nft_mint.ts` and update:

```typescript
// Paste the metadata URI obtained in the previous step
const metadataUri = "https://gateway.irys.xyz/YOUR_METADATA_URI";

let tx = createNft(umi, {
    mint: mint,
    name: "Your NFT Name",
    symbol: "SYMB",
    uri: metadataUri,
    sellerFeeBasisPoints: percentAmount(0),
});
```

Then run:

```bash
yarn nft_mint
# Output:
# Succesfully Minted! Check out your TX here:
# https://explorer.solana.com/tx/...?cluster=devnet
# Mint Address: ...
```

### 8. Verify

- Open the Solscan link to view your transaction: `https://solscan.io/tx/<TX_HASH>?cluster=devnet`
- Search your Mint Address on https://solscan.io/?cluster=devnet to see your NFT

### Flow Summary

```
PNG Image ──► nft_image.ts ──► Image URI (Arweave)
                                      │
                                      ▼
                            nft_metadata.ts ──► Metadata URI (Arweave)
                                                        │
                                                        ▼
                                              nft_mint.ts ──► NFT on Solana devnet
```
