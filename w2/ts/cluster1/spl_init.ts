import { Keypair, Connection, Commitment } from "@solana/web3.js";
import { createMint } from '@solana/spl-token';
import wallet from "../turbin3-wallet.json"

// Import our keypair from the wallet file
const keypair = Keypair.fromSecretKey(new Uint8Array(wallet));

//Create a Solana devnet connection
const commitment: Commitment = "confirmed";
const connection = new Connection("https://api.devnet.solana.com", commitment);

(async () => {
    try {
        const mint = await createMint(
            connection,
            keypair,           // payer
            keypair.publicKey, // mint authority
            null,              // freeze authority (null = no freeze)
            6                  // decimals (6 is standard for tokens)
        );
        console.log(`Your mint address is: ${mint.toBase58()}`);
    } catch(error) {
        console.log(`Oops, something went wrong: ${error}`)
    }
})()
