import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import {
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
  getMint,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";
import { Keypair, PublicKey } from "@solana/web3.js";

describe("amm", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Amm as Program<Amm>;
  const connection = provider.connection;
  const payer = (provider.wallet as anchor.Wallet).payer;

  const SEED = new BN(1);
  const FEE = 100; // 100 bps = 1%

  let mintX: PublicKey;
  let mintY: PublicKey;
  let configPda: PublicKey;
  let mintLpPda: PublicKey;
  let userX: PublicKey;
  let userY: PublicKey;

  before("setup mints and fund user", async () => {
    mintX = await createMint(connection, payer, payer.publicKey, null, 6);
    mintY = await createMint(connection, payer, payer.publicKey, null, 6);

    [configPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("config"),
        mintX.toBuffer(),
        mintY.toBuffer(),
        SEED.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    [mintLpPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), configPda.toBuffer()],
      program.programId
    );

    userX = await createAssociatedTokenAccount(
      connection,
      payer,
      mintX,
      payer.publicKey
    );
    userY = await createAssociatedTokenAccount(
      connection,
      payer,
      mintY,
      payer.publicKey
    );

    await mintTo(connection, payer, mintX, userX, payer, 2_000_000);
    await mintTo(connection, payer, mintY, userY, payer, 2_000_000);
  });

  it("initialize — crée la config et les vaults vides", async () => {
    await program.methods
      .initialize(SEED, FEE, null)
      .accounts({
        initializer: payer.publicKey,
        mintX,
        mintY,
      })
      .signers([payer])
      .rpc();

    const config = await program.account.config.fetch(configPda);
    assert.equal(config.seed.toString(), SEED.toString(), "seed");
    assert.equal(config.fee, FEE, "fee");
    assert.isNull(config.authority, "authority doit être null");
    assert.equal(config.mintX.toString(), mintX.toString(), "mint_x");
    assert.equal(config.mintY.toString(), mintY.toString(), "mint_y");
    assert.isFalse(config.locked, "pool ne doit pas être verrouillé");

    const lpMint = await getMint(connection, mintLpPda);
    assert.equal(lpMint.supply.toString(), "0", "supply LP doit être 0");
  });

  it("deposit — alimente les vaults et frappe des LP tokens", async () => {
    const LP_AMOUNT = new BN(1_000);

    await program.methods
      .deposit(LP_AMOUNT, LP_AMOUNT, LP_AMOUNT)
      .accounts({
        user: payer.publicKey,
        mintX,
        mintY,
        config: configPda,
      })
      .signers([payer])
      .rpc();

    const vaultX = await getAssociatedTokenAddress(mintX, configPda);
    const vaultY = await getAssociatedTokenAddress(mintY, configPda);
    const userLp = await getAssociatedTokenAddress(mintLpPda, payer.publicKey);

    assert.equal(
      (await getAccount(connection, vaultX)).amount.toString(),
      LP_AMOUNT.toString(),
      "vault_x doit contenir les tokens X déposés"
    );
    assert.equal(
      (await getAccount(connection, vaultY)).amount.toString(),
      LP_AMOUNT.toString(),
      "vault_y doit contenir les tokens Y déposés"
    );

    const lpMint = await getMint(connection, mintLpPda);
    assert.equal(
      lpMint.supply.toString(),
      LP_AMOUNT.toString(),
      "supply LP doit égaler le montant déposé"
    );

    assert.equal(
      (await getAccount(connection, userLp)).amount.toString(),
      LP_AMOUNT.toString(),
      "user doit détenir les LP tokens frappés"
    );
  });

  it("withdraw — retire les tokens et brûle les LP", async () => {
    const BURN_AMOUNT = new BN(500);

    const vaultX = await getAssociatedTokenAddress(mintX, configPda);
    const vaultY = await getAssociatedTokenAddress(mintY, configPda);
    const userLp = await getAssociatedTokenAddress(mintLpPda, payer.publicKey);

    const vaultXBefore = (await getAccount(connection, vaultX)).amount;
    const vaultYBefore = (await getAccount(connection, vaultY)).amount;
    const lpSupplyBefore = (await getMint(connection, mintLpPda)).supply;
    const userLpBefore = (await getAccount(connection, userLp)).amount;

    await program.methods
      .withdraw(BURN_AMOUNT, new BN(0), new BN(0))
      .accounts({
        user: payer.publicKey,
        mintX,
        mintY,
        config: configPda,
      })
      .signers([payer])
      .rpc();

    const lpSupplyAfter = (await getMint(connection, mintLpPda)).supply;
    assert.equal(
      lpSupplyAfter.toString(),
      (lpSupplyBefore - BigInt(BURN_AMOUNT.toString())).toString(),
      "supply LP doit diminuer du montant brûlé"
    );

    assert.equal(
      (await getAccount(connection, userLp)).amount.toString(),
      (userLpBefore - BigInt(BURN_AMOUNT.toString())).toString(),
      "balance LP de l'user doit diminuer"
    );

    assert.isTrue(
      (await getAccount(connection, vaultX)).amount < vaultXBefore,
      "vault_x doit diminuer après le retrait"
    );
    assert.isTrue(
      (await getAccount(connection, vaultY)).amount < vaultYBefore,
      "vault_y doit diminuer après le retrait"
    );
  });

  it("swap X→Y — invariant xy=k respecté", async () => {
    const SWAP_AMOUNT = new BN(1_000);

    const vaultX = await getAssociatedTokenAddress(mintX, configPda);
    const vaultY = await getAssociatedTokenAddress(mintY, configPda);

    const vaultXBefore = (await getAccount(connection, vaultX)).amount;
    const vaultYBefore = (await getAccount(connection, vaultY)).amount;
    const userXBefore = (await getAccount(connection, userX)).amount;

    await program.methods
      .swap(true, SWAP_AMOUNT, new BN(1))
      .accounts({
        user: payer.publicKey,
        mintX,
        mintY,
        config: configPda,
      })
      .signers([payer])
      .rpc();

    const vaultXAfter = (await getAccount(connection, vaultX)).amount;
    const vaultYAfter = (await getAccount(connection, vaultY)).amount;

    assert.equal(
      vaultXAfter.toString(),
      (vaultXBefore + BigInt(SWAP_AMOUNT.toString())).toString(),
      "vault_x doit augmenter du montant swappé"
    );
    assert.isTrue(vaultYAfter < vaultYBefore, "vault_y doit diminuer");

    assert.equal(
      (await getAccount(connection, userX)).amount.toString(),
      (userXBefore - BigInt(SWAP_AMOUNT.toString())).toString(),
      "balance X de l'user doit diminuer du montant swappé"
    );

    // invariant xy=k : k_après >= k_avant (les fees restent dans le pool)
    const kBefore = vaultXBefore * vaultYBefore;
    const kAfter = vaultXAfter * vaultYAfter;
    assert.isTrue(kAfter >= kBefore, "xy=k : k ne doit pas diminuer après un swap");
  });
});

// Helper : dérive l'ATA sans créer de compte
function getAssociatedTokenAddress(
  mint: PublicKey,
  owner: PublicKey
): PublicKey {
  const [ata] = PublicKey.findProgramAddressSync(
    [owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
  );
  return ata;
}
