import wallet from "../turbin3-wallet.json"
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults"
import { createGenericFile, createSignerFromKeypair, signerIdentity } from "@metaplex-foundation/umi"
import { irysUploader } from "@metaplex-foundation/umi-uploader-irys"

// Create a devnet connection
const umi = createUmi('https://api.devnet.solana.com');

let keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
const signer = createSignerFromKeypair(umi, keypair);

umi.use(irysUploader());
umi.use(signerIdentity(signer));

(async () => {
    try {
        // Follow this JSON structure
        // https://docs.metaplex.com/programs/token-metadata/changelog/v1.0#json-structure

        // Replace with the image URI obtained from nft_image.ts
        const image = "https://gateway.irys.xyz/7nUoK9js26UjMMy6MQZYRTNunFPEisCqjAZwUiXEyHy4";

        const metadata = {
            name: "Kevred Scientist",
            symbol: "KVRDSCI",
            description: "An ermine dressed as a scientist - Turbin3 Q1 2026",
            image: image,
            attributes: [
                { trait_type: "Species", value: "Hermine" },
                { trait_type: "Outfit", value: "Scientist" },
                { trait_type: "Collection", value: "Turbin3 W3" }
            ],
            properties: {
                files: [
                    {
                        type: "image/png",
                        uri: image
                    },
                ]
            },
            creators: []
        };

        const myUri = await umi.uploader.uploadJson(metadata);
        console.log("Your metadata URI: ", myUri);
    }
    catch(error) {
        console.log("Oops.. Something went wrong", error);
    }
})();
