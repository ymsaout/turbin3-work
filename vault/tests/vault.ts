import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Vault } from "../target/types/vault";
import { expect } from "chai";
import {
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";

describe("vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.vault as Program<Vault>;
  const user = provider.wallet;
  const connection = provider.connection;

  let vaultStatePda: PublicKey;
  let vaultStateBump: number;
  let vaultPda: PublicKey;
  let vaultBump: number;

  before(async () => {
    // Derive the vault_state PDA
    [vaultStatePda, vaultStateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.publicKey.toBuffer()],
      program.programId
    );

    // Derive the vault PDA
    [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), vaultStatePda.toBuffer()],
      program.programId
    );
  });

  it("Initialize", async () => {
    const tx = await program.methods
      .initialize()
      .accountsPartial({
        user: user.publicKey,
        vaultState: vaultStatePda,
        vault: vaultPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("  Initialize tx:", tx);

    // Verify the vault state account was created
    const vaultStateAccount = await program.account.vaultState.fetch(
      vaultStatePda
    );
    expect(vaultStateAccount.vaultBump).to.equal(vaultBump);
    expect(vaultStateAccount.stateBump).to.equal(vaultStateBump);

    console.log("  Vault state bump:", vaultStateAccount.stateBump);
    console.log("  Vault bump:", vaultStateAccount.vaultBump);
  });

  it("Deposit", async () => {
    const depositAmount = 2 * LAMPORTS_PER_SOL;

    const vaultBalanceBefore = await connection.getBalance(vaultPda);

    const tx = await program.methods
      .deposit(new anchor.BN(depositAmount))
      .accountsPartial({
        user: user.publicKey,
        vault: vaultPda,
        vaultState: vaultStatePda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("  Deposit tx:", tx);

    const vaultBalanceAfter = await connection.getBalance(vaultPda);
    expect(vaultBalanceAfter).to.equal(vaultBalanceBefore + depositAmount);

    console.log(
      "  Vault balance:",
      vaultBalanceAfter / LAMPORTS_PER_SOL,
      "SOL"
    );
  });

  it("Withdraw", async () => {
    const withdrawAmount = 1 * LAMPORTS_PER_SOL;

    const vaultBalanceBefore = await connection.getBalance(vaultPda);

    const tx = await program.methods
      .withdraw(new anchor.BN(withdrawAmount))
      .accountsPartial({
        user: user.publicKey,
        vault: vaultPda,
        vaultState: vaultStatePda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("  Withdraw tx:", tx);

    const vaultBalanceAfter = await connection.getBalance(vaultPda);
    expect(vaultBalanceAfter).to.equal(vaultBalanceBefore - withdrawAmount);

    console.log(
      "  Vault balance after withdraw:",
      vaultBalanceAfter / LAMPORTS_PER_SOL,
      "SOL"
    );
  });

  it("Close", async () => {
    const userBalanceBefore = await connection.getBalance(user.publicKey);
    const vaultBalanceBefore = await connection.getBalance(vaultPda);

    const tx = await program.methods
      .close()
      .accountsPartial({
        user: user.publicKey,
        vault: vaultPda,
        vaultState: vaultStatePda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("  Close tx:", tx);

    const vaultBalanceAfter = await connection.getBalance(vaultPda);
    expect(vaultBalanceAfter).to.equal(0);

    // Check the vault state account is closed
    const vaultStateAccount = await connection.getAccountInfo(vaultStatePda);
    expect(vaultStateAccount).to.be.null;

    const userBalanceAfter = await connection.getBalance(user.publicKey);
    console.log(
      "  User recovered:",
      (userBalanceAfter - userBalanceBefore) / LAMPORTS_PER_SOL,
      "SOL (minus tx fees)"
    );
    console.log("  Vault balance after close:", vaultBalanceAfter, "lamports");
    console.log("  Vault state account closed:", vaultStateAccount === null);
  });
});
