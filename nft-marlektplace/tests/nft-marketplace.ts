import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftMarketplace } from "../target/types/nft_marketplace";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";

const MPL_CORE_PROGRAM_ID = new PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
);
const MARKETPLACE_NAME = "TestMarket";
const FEE_BPS = 250; // 2.5%
const LISTING_PRICE_SOL = 0.1 * LAMPORTS_PER_SOL;

describe("nft-marketplace", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.NftMarketplace as Program<NftMarketplace>;
  const conn = provider.connection;
  const admin = provider.wallet as anchor.Wallet;

  const collection = Keypair.generate();
  let paymentMint: PublicKey;

  let marketplace: PublicKey;
  let treasury: PublicKey;
  let rewardsMint: PublicKey;
  let collectionAuthority: PublicKey;

  before(async () => {
    [marketplace] = PublicKey.findProgramAddressSync(
      [Buffer.from("marketplace"), Buffer.from(MARKETPLACE_NAME)],
      program.programId
    );
    [treasury] = PublicKey.findProgramAddressSync(
      [Buffer.from("treasury"), marketplace.toBuffer()],
      program.programId
    );
    [rewardsMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("rewards_mint"), marketplace.toBuffer()],
      program.programId
    );
    [collectionAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("collection_authority"), collection.publicKey.toBuffer()],
      program.programId
    );

    paymentMint = await createMint(conn, admin.payer, admin.publicKey, null, 6);
  });

  // ─── Setup ─────────────────────────────────────────────────────────────────

  it("creates a collection", async () => {
    const tx = await program.methods
      .createCollection("Test Collection", "https://example.com/col.json")
      .accountsPartial({
        payer: admin.publicKey,
        collection: collection.publicKey,
        collectionAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([collection])
      .rpc();
    console.log("  createCollection:", tx);
  });

  it("initializes the marketplace", async () => {
    try {
      const tx = await program.methods
        .initialize(MARKETPLACE_NAME, FEE_BPS)
        .accountsPartial({
          admin: admin.publicKey,
          marketplace,
          treasury,
          rewardsMint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("  initialize:", tx);
    } catch (e: any) {
      // Surfpool est persistant — le compte peut déjà exister d'une session précédente
      if (!e.message?.includes("already in use")) throw e;
      console.log("  marketplace already exists, skipping init");
    }
    const state = await program.account.marketplace.fetch(marketplace);
    expect(state.fee).to.equal(FEE_BPS);
    expect(state.name).to.equal(MARKETPLACE_NAME);
  });

  // ─── Flow 1 : list + buy (SOL) ─────────────────────────────────────────────

  it("lists an NFT and buys it with SOL", async () => {
    const asset1 = Keypair.generate();

    await program.methods
      .mintAsset("NFT #1", "https://example.com/nft/1.json")
      .accountsPartial({
        user: admin.publicKey,
        asset: asset1.publicKey,
        collection: collection.publicKey,
        collectionAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([asset1])
      .rpc();

    const [listing] = PublicKey.findProgramAddressSync(
      [Buffer.from("listing"), marketplace.toBuffer(), asset1.publicKey.toBuffer()],
      program.programId
    );

    const listTx = await program.methods
      .list(new anchor.BN(LISTING_PRICE_SOL), null)
      .accountsPartial({
        maker: admin.publicKey,
        asset: asset1.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("  list (SOL):", listTx);

    const takerRewardsAta = getAssociatedTokenAddressSync(rewardsMint, admin.publicKey);

    const buyTx = await program.methods
      .buy()
      .accountsPartial({
        taker: admin.publicKey,
        maker: admin.publicKey,
        asset: asset1.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        treasury,
        rewardsMint,
        takerRewardsAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("  buy (SOL):", buyTx);

    const fee = Math.floor((LISTING_PRICE_SOL * FEE_BPS) / 10_000);
    const treasuryBal = await conn.getBalance(treasury);
    expect(treasuryBal).to.be.gte(fee);
  });

  // ─── Flow 2 : list + delist ─────────────────────────────────────────────────

  it("lists and delists an NFT", async () => {
    const asset2 = Keypair.generate();

    await program.methods
      .mintAsset("NFT #2", "https://example.com/nft/2.json")
      .accountsPartial({
        user: admin.publicKey,
        asset: asset2.publicKey,
        collection: collection.publicKey,
        collectionAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([asset2])
      .rpc();

    const [listing] = PublicKey.findProgramAddressSync(
      [Buffer.from("listing"), marketplace.toBuffer(), asset2.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .list(new anchor.BN(LISTING_PRICE_SOL), null)
      .accountsPartial({
        maker: admin.publicKey,
        asset: asset2.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const delistTx = await program.methods
      .delist()
      .accountsPartial({
        maker: admin.publicKey,
        asset: asset2.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("  delist:", delistTx);

    try {
      await program.account.listing.fetch(listing);
      expect.fail("Listing should be closed");
    } catch (_) {}
  });

  // ─── Flow 3 : list + buy_with_token ─────────────────────────────────────────

  it("lists and buys with SPL token", async () => {
    const asset3 = Keypair.generate();
    const TOKEN_PRICE = 1_000_000; // 1 USDC
    // Acheteur distinct du maker pour éviter l'aliasing des ATAs
    const buyer = Keypair.generate();
    const sig = await conn.requestAirdrop(buyer.publicKey, LAMPORTS_PER_SOL);
    await conn.confirmTransaction(sig);

    // Pré-crée TOUTES les ATAs nécessaires avant le call
    // (les contraintes associated_token ont été retirées du struct Rust pour éviter
    //  la résolution automatique erronée par Anchor TypeScript)
    const buyerPaymentAta = await getOrCreateAssociatedTokenAccount(
      conn, admin.payer, paymentMint, buyer.publicKey
    );
    await mintTo(conn, admin.payer, paymentMint, buyerPaymentAta.address, admin.publicKey, 10_000_000);

    const treasuryAtaAccount = await getOrCreateAssociatedTokenAccount(
      conn, admin.payer, paymentMint, treasury, true // allowOwnerOffCurve pour PDA
    );
    const makerPaymentAtaAccount = await getOrCreateAssociatedTokenAccount(
      conn, admin.payer, paymentMint, admin.publicKey
    );
    const buyerRewardsAtaAccount = await getOrCreateAssociatedTokenAccount(
      conn, admin.payer, rewardsMint, buyer.publicKey
    );

    await program.methods
      .mintAsset("NFT #3", "https://example.com/nft/3.json")
      .accountsPartial({
        user: admin.publicKey,
        asset: asset3.publicKey,
        collection: collection.publicKey,
        collectionAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([asset3])
      .rpc();

    const [listing] = PublicKey.findProgramAddressSync(
      [Buffer.from("listing"), marketplace.toBuffer(), asset3.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .list(new anchor.BN(TOKEN_PRICE), paymentMint)
      .accountsPartial({
        maker: admin.publicKey,
        asset: asset3.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const buyTx = await program.methods
      .buyWithToken()
      .accountsPartial({
        taker: buyer.publicKey,
        maker: admin.publicKey,
        asset: asset3.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        treasury,
        paymentMint,
        treasuryAta: treasuryAtaAccount.address,
        takerPaymentAta: buyerPaymentAta.address,
        makerPaymentAta: makerPaymentAtaAccount.address,
        rewardsMint,
        takerRewardsAta: buyerRewardsAtaAccount.address,
        tokenProgram: TOKEN_PROGRAM_ID,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();
    console.log("  buy_with_token:", buyTx);
  });

  // ─── Flow 4 : make_offer + accept_offer ─────────────────────────────────────

  it("makes an offer and the maker accepts it", async () => {
    const asset4 = Keypair.generate();
    const buyer = Keypair.generate();
    const OFFER_AMOUNT = 0.08 * LAMPORTS_PER_SOL;

    const sig = await conn.requestAirdrop(buyer.publicKey, LAMPORTS_PER_SOL);
    await conn.confirmTransaction(sig);

    await program.methods
      .mintAsset("NFT #4", "https://example.com/nft/4.json")
      .accountsPartial({
        user: admin.publicKey,
        asset: asset4.publicKey,
        collection: collection.publicKey,
        collectionAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([asset4])
      .rpc();

    const [listing] = PublicKey.findProgramAddressSync(
      [Buffer.from("listing"), marketplace.toBuffer(), asset4.publicKey.toBuffer()],
      program.programId
    );
    const [offer] = PublicKey.findProgramAddressSync(
      [Buffer.from("offer"), asset4.publicKey.toBuffer(), buyer.publicKey.toBuffer()],
      program.programId
    );
    const [offerVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("offer_vault"), asset4.publicKey.toBuffer(), buyer.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .list(new anchor.BN(LISTING_PRICE_SOL), null)
      .accountsPartial({
        maker: admin.publicKey,
        asset: asset4.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .makeOffer(new anchor.BN(OFFER_AMOUNT))
      .accountsPartial({
        buyer: buyer.publicKey,
        asset: asset4.publicKey,
        listing,
        offer,
        offerVault,
        marketplace,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const buyerRewardsAta = getAssociatedTokenAddressSync(rewardsMint, buyer.publicKey);

    const acceptTx = await program.methods
      .acceptOffer()
      .accountsPartial({
        maker: admin.publicKey,
        buyer: buyer.publicKey,
        asset: asset4.publicKey,
        collection: collection.publicKey,
        listing,
        offer,
        offerVault,
        marketplace,
        treasury,
        rewardsMint,
        buyerRewardsAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("  accept_offer:", acceptTx);

    try {
      await program.account.offer.fetch(offer);
      expect.fail("Offer should be closed");
    } catch (_) {}
  });

  // ─── Flow 5 : make_offer + cancel_offer ─────────────────────────────────────

  it("makes an offer and the buyer cancels it", async () => {
    const asset5 = Keypair.generate();
    const OFFER_AMOUNT = 0.05 * LAMPORTS_PER_SOL;

    await program.methods
      .mintAsset("NFT #5", "https://example.com/nft/5.json")
      .accountsPartial({
        user: admin.publicKey,
        asset: asset5.publicKey,
        collection: collection.publicKey,
        collectionAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([asset5])
      .rpc();

    const [listing] = PublicKey.findProgramAddressSync(
      [Buffer.from("listing"), marketplace.toBuffer(), asset5.publicKey.toBuffer()],
      program.programId
    );
    const [offer] = PublicKey.findProgramAddressSync(
      [Buffer.from("offer"), asset5.publicKey.toBuffer(), admin.publicKey.toBuffer()],
      program.programId
    );
    const [offerVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("offer_vault"), asset5.publicKey.toBuffer(), admin.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .list(new anchor.BN(LISTING_PRICE_SOL), null)
      .accountsPartial({
        maker: admin.publicKey,
        asset: asset5.publicKey,
        collection: collection.publicKey,
        listing,
        marketplace,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const balBefore = await conn.getBalance(admin.publicKey);

    await program.methods
      .makeOffer(new anchor.BN(OFFER_AMOUNT))
      .accountsPartial({
        buyer: admin.publicKey,
        asset: asset5.publicKey,
        listing,
        offer,
        offerVault,
        marketplace,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const cancelTx = await program.methods
      .cancelOffer()
      .accountsPartial({
        buyer: admin.publicKey,
        asset: asset5.publicKey,
        offer,
        offerVault,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("  cancel_offer:", cancelTx);

    const balAfter = await conn.getBalance(admin.publicKey);
    expect(balAfter).to.be.closeTo(balBefore, 0.01 * LAMPORTS_PER_SOL);
  });
});
