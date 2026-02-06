import { Keypair, PublicKey, Connection, Commitment } from "@solana/web3.js";
import { getOrCreateAssociatedTokenAccount, mintTo } from '@solana/spl-token';
import wallet from "../turbin3-wallet.json"

// Import our keypair from the wallet file
const keypair = Keypair.fromSecretKey(new Uint8Array(wallet));

//Create a Solana devnet connection
const commitment: Commitment = "confirmed";
const connection = new Connection("https://api.devnet.solana.com", commitment);

const token_decimals = 1_000_000n;

// Mint address
const mint = new PublicKey("53uQFaPsd5ADTUmXMahxW63TZttyLtyuwqJWqiQPHJv6");

(async () => {
    try {
        // Create an ATA
        const ata = await getOrCreateAssociatedTokenAccount(
            connection,
            keypair,           // payer
            mint,              // mint address
            keypair.publicKey  // owner
        );
        console.log(`Your ata is: ${ata.address.toBase58()}`);

        // Mint to ATA (100 tokens with 6 decimals)
        const mintTx = await mintTo(
            connection,
            keypair,           // payer
            mint,              // mint address
            ata.address,       // destination
            keypair,           // mint authority
            100n * token_decimals
        );
        console.log(`Your mint txid: ${mintTx}`);
    } catch(error) {
        console.log(`Oops, something went wrong: ${error}`)
    }
})()
