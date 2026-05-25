import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Escrow } from "../target/types/escrow";
import { expect } from "chai";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";

describe("escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.escrow as Program<Escrow>;
  const connection = provider.connection;
  const makerWallet = provider.wallet;

  let escrowStatePdaForTake: PublicKey;
  let vaultPdaForTake: PublicKey;

  before(async () => {
    // PDAs for the flow where a taker accepts the escrow.
    [escrowStatePdaForTake] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), makerWallet.publicKey.toBuffer()],
      program.programId
    );

    [vaultPdaForTake] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), escrowStatePdaForTake.toBuffer()],
      program.programId
    );
  });

  it("make - locks lamports in the vault PDA", async () => {
    const amount = 1 * LAMPORTS_PER_SOL;

    const tx = await program.methods
      .make(new anchor.BN(amount))
      .accountsPartial({
        maker: makerWallet.publicKey,
        escrowState: escrowStatePdaForTake,
        vault: vaultPdaForTake,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("  Make tx:", tx);

    const vaultBalance = await connection.getBalance(vaultPdaForTake);
    expect(vaultBalance).to.equal(amount);

    const state = await program.account.escrowState.fetch(escrowStatePdaForTake);
    expect(state.maker.toBase58()).to.equal(
      makerWallet.publicKey.toBase58()
    );
    expect(state.amount.toNumber()).to.equal(amount);
  });

  it("take - taker pays maker and receives the locked lamports", async () => {
    const taker = Keypair.generate();

    // Fund the taker so they can pay the maker.
    const airdropSig = await connection.requestAirdrop(
      taker.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSig);

    const amount = 1 * LAMPORTS_PER_SOL;

    const makerBalanceBefore = await connection.getBalance(
      makerWallet.publicKey
    );
    const takerBalanceBefore = await connection.getBalance(taker.publicKey);

    const tx = await program.methods
      .take()
      .accountsPartial({
        taker: taker.publicKey,
        maker: makerWallet.publicKey,
        escrowState: escrowStatePdaForTake,
        vault: vaultPdaForTake,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    console.log("  Take tx:", tx);

    const makerBalanceAfter = await connection.getBalance(
      makerWallet.publicKey
    );
    const takerBalanceAfter = await connection.getBalance(taker.publicKey);
    const vaultBalanceAfter = await connection.getBalance(vaultPdaForTake);

    // Maker should have more lamports than before (taker paid them).
    expect(makerBalanceAfter).to.be.greaterThan(makerBalanceBefore);

    // Vault should be emptied.
    expect(vaultBalanceAfter).to.equal(0);

    // Escrow state account should be closed.
    const escrowStateAccount = await connection.getAccountInfo(
      escrowStatePdaForTake
    );
    expect(escrowStateAccount).to.be.null;

    console.log(
      "  Taker balance change (includes tx fees):",
      (takerBalanceAfter - takerBalanceBefore) / LAMPORTS_PER_SOL,
      "SOL"
    );
  });

  it("refund - maker recovers locked lamports", async () => {
    // Use a fresh maker for the refund flow so we can create a new escrow.
    const makerForRefund = Keypair.generate();

    const airdropSig = await connection.requestAirdrop(
      makerForRefund.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSig);

    const [escrowStatePdaForRefund] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), makerForRefund.publicKey.toBuffer()],
      program.programId
    );

    const [vaultPdaForRefund] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), escrowStatePdaForRefund.toBuffer()],
      program.programId
    );

    const amount = 1 * LAMPORTS_PER_SOL;

    // First, create the escrow for this maker.
    await program.methods
      .make(new anchor.BN(amount))
      .accountsPartial({
        maker: makerForRefund.publicKey,
        escrowState: escrowStatePdaForRefund,
        vault: vaultPdaForRefund,
        systemProgram: SystemProgram.programId,
      })
      .signers([makerForRefund])
      .rpc();

    const makerBalanceBefore = await connection.getBalance(
      makerForRefund.publicKey
    );
    const vaultBalanceBefore = await connection.getBalance(vaultPdaForRefund);

    const tx = await program.methods
      .refund()
      .accountsPartial({
        maker: makerForRefund.publicKey,
        escrowState: escrowStatePdaForRefund,
        vault: vaultPdaForRefund,
        systemProgram: SystemProgram.programId,
      })
      .signers([makerForRefund])
      .rpc();

    console.log("  Refund tx:", tx);

    const makerBalanceAfter = await connection.getBalance(
      makerForRefund.publicKey
    );
    const vaultBalanceAfter = await connection.getBalance(vaultPdaForRefund);

    // Vault should be emptied.
    expect(vaultBalanceAfter).to.equal(0);

    // Escrow state account should be closed.
    const escrowStateAccount = await connection.getAccountInfo(
      escrowStatePdaForRefund
    );
    expect(escrowStateAccount).to.be.null;

    // Maker should have recovered funds from the vault (minus tx fees).
    expect(makerBalanceAfter).to.be.greaterThan(makerBalanceBefore);

    console.log(
      "  Maker recovered (minus fees):",
      (makerBalanceAfter - makerBalanceBefore + vaultBalanceBefore) /
        LAMPORTS_PER_SOL,
      "SOL"
    );
  });
});
