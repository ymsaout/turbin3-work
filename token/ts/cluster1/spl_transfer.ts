import { Commitment, Connection, Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js"
import wallet from "../turbin3-wallet.json"
import { getOrCreateAssociatedTokenAccount, transfer } from "@solana/spl-token";

// We're going to import our keypair from the wallet file
const keypair = Keypair.fromSecretKey(new Uint8Array(wallet));

//Create a Solana devnet connection
const commitment: Commitment = "confirmed";
const connection = new Connection("https://api.devnet.solana.com", commitment);

// Mint address
const mint = new PublicKey("53uQFaPsd5ADTUmXMahxW63TZttyLtyuwqJWqiQPHJv6");

// Recipient address
const to = new PublicKey("urAKh83tZRMp248mt94g2rNqcuo45SaxJoTPUit381p");

const token_decimals = 1_000_000n;

(async () => {
    try {
        // Get the token account of the fromWallet address, and if it does not exist, create it
        const fromAta = await getOrCreateAssociatedTokenAccount(
            connection,
            keypair,
            mint,
            keypair.publicKey
        );

        // Get the token account of the toWallet address, and if it does not exist, create it
        const toAta = await getOrCreateAssociatedTokenAccount(
            connection,
            keypair,           // payer (you pay the fees)
            mint,
            to                 // owner of the destination account
        );

        // Transfer 3 tokens
        const tx = await transfer(
            connection,
            keypair,           // payer
            fromAta.address,   // source
            toAta.address,     // destination
            keypair,           // owner of source account
            3n * token_decimals
        );
        console.log(`Transfer txid: ${tx}`);
    } catch(e) {
        console.error(`Oops, something went wrong: ${e}`)
    }
})();