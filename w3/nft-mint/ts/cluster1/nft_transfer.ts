import { createUmi } from "@metaplex-foundation/umi-bundle-defaults"
import { createSignerFromKeypair, signerIdentity, publicKey } from "@metaplex-foundation/umi"
import { transferV1, mplTokenMetadata, TokenStandard } from "@metaplex-foundation/mpl-token-metadata";

import wallet from "../turbin3-wallet.json"
import base58 from "bs58";

const RPC_ENDPOINT = "https://api.devnet.solana.com";
const umi = createUmi(RPC_ENDPOINT);

let keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
const myKeypairSigner = createSignerFromKeypair(umi, keypair);
umi.use(signerIdentity(myKeypairSigner));
umi.use(mplTokenMetadata());

// NFT mint address
const mint = publicKey("8cgf55BtX1EKjeXb1nmqtdLokSete2NYnyWLF1hFLYji");

// Recipient address
const destination = publicKey("MD25vTeJ4XM2qsd4nX9EEhgP4sHUC5nwDYLpN3TDhNE");

(async () => {
    try {
        let tx = transferV1(umi, {
            mint: mint,
            destinationOwner: destination,
            amount: 1,
            tokenStandard: TokenStandard.NonFungible,
        });

        let result = await tx.sendAndConfirm(umi);
        const signature = base58.encode(result.signature);

        console.log(`Successfully Transferred! Check out your TX here:\nhttps://explorer.solana.com/tx/${signature}?cluster=devnet`);
    } catch (error) {
        console.error("Oops, something went wrong:", error);
    }
})();
