import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftStaking } from "../target/types/nft_staking";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";

const MPL_CORE_PROGRAM_ID = new PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
);

describe("nft-staking", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.NftStaking as Program<NftStaking>;
  const admin = provider.wallet as anchor.Wallet;

  const collection = Keypair.generate();
  const asset = Keypair.generate();

  let updateAuthority: PublicKey;
  let config: PublicKey;
  let rewardsMint: PublicKey;
  let userRewardsAta: PublicKey;

  before(() => {
    [updateAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("update_authority"), collection.publicKey.toBuffer()],
      program.programId
    );

    [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), collection.publicKey.toBuffer()],
      program.programId
    );

    [rewardsMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("rewards_mint"), collection.publicKey.toBuffer()],
      program.programId
    );

    userRewardsAta = getAssociatedTokenAddressSync(
      rewardsMint,
      admin.publicKey
    );
  });

  it("creates a collection", async () => {
    const tx = await program.methods
      .createCollection("Test Collection", "https://example.com/collection.json")
      .accountsPartial({
        payer: admin.publicKey,
        collection: collection.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([collection])
      .rpc();

    console.log("  createCollection:", tx);
  });

  it("initializes the config", async () => {
    // freeze_period = 0 days pour pouvoir unstake immédiatement en test
    const tx = await program.methods
      .initialize(500, 0)
      .accountsPartial({
        admin: admin.publicKey,
        config,
        collection: collection.publicKey,
        updateAuthority,
        rewardsMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("  initialize:", tx);

    const state = await program.account.config.fetch(config);
    expect(state.rewardsBasisPoints).to.equal(500);
    expect(state.freezePeriod).to.equal(0);
  });

  it("mints an asset into the collection", async () => {
    const tx = await program.methods
      .mintAsset("Test NFT #1", "https://example.com/nft/1.json")
      .accountsPartial({
        user: admin.publicKey,
        asset: asset.publicKey,
        collection: collection.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([asset])
      .rpc();

    console.log("  mintAsset:", tx);
  });

  it("stakes the asset", async () => {
    const tx = await program.methods
      .stake()
      .accountsPartial({
        owner: admin.publicKey,
        config,
        asset: asset.publicKey,
        collection: collection.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc();

    console.log("  stake:", tx);
  });

  it("unstakes the asset and claims rewards", async () => {
    const tx = await program.methods
      .unstake()
      .accountsPartial({
        owner: admin.publicKey,
        config,
        asset: asset.publicKey,
        collection: collection.publicKey,
        updateAuthority,
        rewardsMint,
        userRewardsAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc();

    console.log("  unstake:", tx);
  });
});
