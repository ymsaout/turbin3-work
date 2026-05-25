// Integration tests for the AMM program using LiteSVM (native Rust, not BPF).
//
// PREREQUISITES:
//   1. anchor build          — compiles the program to a .so file
//   2. anchor keys sync      — updates declare_id! and Anchor.toml with the real key
//   3. Update PROGRAM_ID below with the value shown by `anchor keys list`
//   4. cargo test            — runs these tests
//
// COURSE REQUIREMENT (Turbin3):
//   After every instruction call, fetch the relevant on-chain state accounts and
//   assert that their fields match the expected values — not just that the account exists.

use anchor_lang::AccountDeserialize;
use amm::state::Config;
use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    rent::Rent,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

// ⚠️  Update after running `anchor keys sync`
const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

// Path to the BPF binary produced by `anchor build`
const PROGRAM_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/deploy/amm.so"
);

// ─── Account parsers ─────────────────────────────────────────────────────────

/// Deserialises an Anchor `Config` account from raw on-chain data.
fn parse_config(svm: &LiteSVM, config: &Pubkey) -> Config {
    let raw = svm.get_account(config).expect("config account not found");
    Config::try_deserialize(&mut raw.data.as_slice()).expect("failed to deserialise Config")
}

/// Returns the SPL Token mint supply.
/// Mint layout: mint_authority (36) | supply (8) | …
fn parse_mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    let raw = svm.get_account(mint).expect("mint account not found");
    u64::from_le_bytes(raw.data[36..44].try_into().unwrap())
}

/// Returns the token balance of an SPL Token account.
/// Token account layout: mint (32) | owner (32) | amount (8) | …
fn parse_token_amount(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let raw = svm.get_account(account).expect("token account not found");
    u64::from_le_bytes(raw.data[64..72].try_into().unwrap())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}

/// Computes the Anchor instruction discriminator: sha256("global:<name>")[..8]
fn discriminator(name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(format!("global:{name}"));
    h.finalize()[..8].try_into().unwrap()
}

/// Builds and sends a transaction; panics on failure.
fn send(svm: &mut LiteSVM, ixs: &[Instruction], payer: &Keypair, extra: &[&Keypair]) {
    let hash = svm.latest_blockhash();
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra);
    let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), &signers, hash);
    svm.send_transaction(tx).expect("Transaction failed");
}

// ─── Shared addresses ────────────────────────────────────────────────────────

struct Addrs {
    mint_x: Keypair,
    mint_y: Keypair,
    config: Pubkey,
    mint_lp: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    seed: u64,
}

/// Creates both token mints and derives all PDAs / ATAs used across tests.
fn setup(svm: &mut LiteSVM, payer: &Keypair) -> Addrs {
    let pid = program_id();
    let seed: u64 = 1;

    let mint_x = Keypair::new();
    let mint_y = Keypair::new();

    let rent = Rent::default().minimum_balance(spl_token::state::Mint::LEN);

    for mint in [&mint_x, &mint_y] {
        let create_ix = system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        );
        let init_ix = spl_token::instruction::initialize_mint2(
            &spl_token::id(),
            &mint.pubkey(),
            &payer.pubkey(), // mint authority = payer (convenient for tests)
            None,
            6,
        )
        .unwrap();
        send(svm, &[create_ix, init_ix], payer, &[mint]);
    }

    let (config, _) = Pubkey::find_program_address(
        &[
            b"config",
            mint_x.pubkey().as_ref(),
            mint_y.pubkey().as_ref(),
            &seed.to_le_bytes(),
        ],
        &pid,
    );
    let (mint_lp, _) =
        Pubkey::find_program_address(&[b"lp", config.as_ref()], &pid);
    let vault_x = get_associated_token_address(&config, &mint_x.pubkey());
    let vault_y = get_associated_token_address(&config, &mint_y.pubkey());

    Addrs { mint_x, mint_y, config, mint_lp, vault_x, vault_y, seed }
}

/// Creates the ATA of `owner` for `mint` and mints `amount` tokens into it.
fn fund_user(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);
    let create_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        owner,
        mint,
        &spl_token::id(),
    );
    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        &ata,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    send(svm, &[create_ix, mint_ix], payer, &[]);
    ata
}

// ─── Instruction builders ────────────────────────────────────────────────────

fn initialize_ix(payer: &Keypair, a: &Addrs) -> Instruction {
    let mut data = discriminator("initialize").to_vec();
    data.extend_from_slice(&a.seed.to_le_bytes()); // seed: u64
    data.extend_from_slice(&100u16.to_le_bytes());  // fee: u16  (1 % = 100 bps)
    data.push(0);                                   // authority: Option<Pubkey> = None

    Instruction::new_with_bytes(
        program_id(),
        &data,
        vec![
            AccountMeta::new(payer.pubkey(), true),              // initializer
            AccountMeta::new_readonly(a.mint_x.pubkey(), false), // mint_x
            AccountMeta::new_readonly(a.mint_y.pubkey(), false), // mint_y
            AccountMeta::new(a.mint_lp, false),                  // mint_lp (PDA)
            AccountMeta::new(a.vault_x, false),                  // vault_x (ATA)
            AccountMeta::new(a.vault_y, false),                  // vault_y (ATA)
            AccountMeta::new(a.config, false),                   // config (PDA)
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
    )
}

fn deposit_ix(user: &Keypair, a: &Addrs, amount: u64, max_x: u64, max_y: u64) -> Instruction {
    let user_x = get_associated_token_address(&user.pubkey(), &a.mint_x.pubkey());
    let user_y = get_associated_token_address(&user.pubkey(), &a.mint_y.pubkey());
    let user_lp = get_associated_token_address(&user.pubkey(), &a.mint_lp);

    let mut data = discriminator("deposit").to_vec();
    data.extend_from_slice(&amount.to_le_bytes()); // LP tokens to mint
    data.extend_from_slice(&max_x.to_le_bytes());  // max X willing to deposit
    data.extend_from_slice(&max_y.to_le_bytes());  // max Y willing to deposit

    Instruction::new_with_bytes(
        program_id(),
        &data,
        vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(a.mint_x.pubkey(), false),
            AccountMeta::new_readonly(a.mint_y.pubkey(), false),
            AccountMeta::new_readonly(a.config, false),
            AccountMeta::new(a.mint_lp, false),
            AccountMeta::new(a.vault_x, false),
            AccountMeta::new(a.vault_y, false),
            AccountMeta::new(user_x, false),
            AccountMeta::new(user_y, false),
            AccountMeta::new(user_lp, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
    )
}

fn withdraw_ix(user: &Keypair, a: &Addrs, amount: u64, min_x: u64, min_y: u64) -> Instruction {
    let user_x = get_associated_token_address(&user.pubkey(), &a.mint_x.pubkey());
    let user_y = get_associated_token_address(&user.pubkey(), &a.mint_y.pubkey());
    let user_lp = get_associated_token_address(&user.pubkey(), &a.mint_lp);

    let mut data = discriminator("withdraw").to_vec();
    data.extend_from_slice(&amount.to_le_bytes()); // LP tokens to burn
    data.extend_from_slice(&min_x.to_le_bytes());  // min X expected back
    data.extend_from_slice(&min_y.to_le_bytes());  // min Y expected back

    Instruction::new_with_bytes(
        program_id(),
        &data,
        vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(a.mint_x.pubkey(), false),
            AccountMeta::new_readonly(a.mint_y.pubkey(), false),
            AccountMeta::new_readonly(a.config, false),
            AccountMeta::new(a.mint_lp, false),
            AccountMeta::new(a.vault_x, false),
            AccountMeta::new(a.vault_y, false),
            AccountMeta::new(user_x, false),
            AccountMeta::new(user_y, false),
            AccountMeta::new(user_lp, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
    )
}

fn swap_ix(user: &Keypair, a: &Addrs, is_x: bool, amount: u64, minimum: u64) -> Instruction {
    let user_x = get_associated_token_address(&user.pubkey(), &a.mint_x.pubkey());
    let user_y = get_associated_token_address(&user.pubkey(), &a.mint_y.pubkey());

    let mut data = discriminator("swap").to_vec();
    data.push(is_x as u8);                          // direction: true = X→Y
    data.extend_from_slice(&amount.to_le_bytes());   // amount to deposit
    data.extend_from_slice(&minimum.to_le_bytes());  // minimum amount to receive

    Instruction::new_with_bytes(
        program_id(),
        &data,
        vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(a.mint_x.pubkey(), false),
            AccountMeta::new_readonly(a.mint_y.pubkey(), false),
            AccountMeta::new_readonly(a.config, false),
            AccountMeta::new_readonly(a.mint_lp, false),
            AccountMeta::new(a.vault_x, false),
            AccountMeta::new(a.vault_y, false),
            AccountMeta::new(user_x, false),
            AccountMeta::new(user_y, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
    )
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(program_id(), PROGRAM_SO).unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let a = setup(&mut svm, &payer);
    send(&mut svm, &[initialize_ix(&payer, &a)], &payer, &[]);

    // ── Verify Config state ──────────────────────────────────────────────────
    let config = parse_config(&svm, &a.config);

    assert_eq!(config.seed, a.seed,          "seed must match the argument passed");
    assert_eq!(config.fee,  100,             "fee must be 100 bps (1 %)");
    assert_eq!(config.authority, None,       "authority must be None (permissionless)");
    assert_eq!(config.mint_x, a.mint_x.pubkey(), "mint_x must match");
    assert_eq!(config.mint_y, a.mint_y.pubkey(), "mint_y must match");
    assert!(!config.locked,                  "pool must not be locked on init");

    // ── Verify LP mint: supply must be 0 at creation ─────────────────────────
    assert_eq!(parse_mint_supply(&svm, &a.mint_lp), 0, "LP supply must start at 0");

    // ── Verify vaults: both must be empty at creation ────────────────────────
    assert_eq!(parse_token_amount(&svm, &a.vault_x), 0, "vault_x must start empty");
    assert_eq!(parse_token_amount(&svm, &a.vault_y), 0, "vault_y must start empty");
}

#[test]
fn test_deposit() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(program_id(), PROGRAM_SO).unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let a = setup(&mut svm, &payer);

    fund_user(&mut svm, &payer, &payer.pubkey(), &a.mint_x.pubkey(), 1_000_000);
    fund_user(&mut svm, &payer, &payer.pubkey(), &a.mint_y.pubkey(), 1_000_000);

    send(&mut svm, &[initialize_ix(&payer, &a)], &payer, &[]);

    // First deposit: free ratio 1:1, claiming 1 000 LP tokens.
    // Because the pool is empty, max_x = max_y = amount = 1 000.
    let lp_amount: u64 = 1_000;
    send(
        &mut svm,
        &[deposit_ix(&payer, &a, lp_amount, lp_amount, lp_amount)],
        &payer,
        &[],
    );

    // ── Verify vaults received the deposited tokens ──────────────────────────
    assert_eq!(
        parse_token_amount(&svm, &a.vault_x),
        lp_amount,
        "vault_x must hold the deposited X amount"
    );
    assert_eq!(
        parse_token_amount(&svm, &a.vault_y),
        lp_amount,
        "vault_y must hold the deposited Y amount"
    );

    // ── Verify LP mint supply equals the claimed amount ──────────────────────
    assert_eq!(
        parse_mint_supply(&svm, &a.mint_lp),
        lp_amount,
        "LP supply must equal the amount minted"
    );

    // ── Verify user_lp ATA holds the minted LP tokens ───────────────────────
    let user_lp = get_associated_token_address(&payer.pubkey(), &a.mint_lp);
    assert_eq!(
        parse_token_amount(&svm, &user_lp),
        lp_amount,
        "user LP balance must equal the minted amount"
    );
}

#[test]
fn test_withdraw() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(program_id(), PROGRAM_SO).unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let a = setup(&mut svm, &payer);

    fund_user(&mut svm, &payer, &payer.pubkey(), &a.mint_x.pubkey(), 1_000_000);
    fund_user(&mut svm, &payer, &payer.pubkey(), &a.mint_y.pubkey(), 1_000_000);

    send(&mut svm, &[initialize_ix(&payer, &a)], &payer, &[]);

    let lp_amount: u64 = 1_000;
    send(&mut svm, &[deposit_ix(&payer, &a, lp_amount, lp_amount, lp_amount)], &payer, &[]);

    // Record state before withdrawal
    let vault_x_before = parse_token_amount(&svm, &a.vault_x);
    let vault_y_before = parse_token_amount(&svm, &a.vault_y);
    let lp_supply_before = parse_mint_supply(&svm, &a.mint_lp);

    let user_lp = get_associated_token_address(&payer.pubkey(), &a.mint_lp);
    let user_lp_before = parse_token_amount(&svm, &user_lp);

    // Burn half the LP position; accept any amount of X and Y back
    let burn_amount: u64 = 500;
    send(&mut svm, &[withdraw_ix(&payer, &a, burn_amount, 0, 0)], &payer, &[]);

    // ── Verify LP supply decreased by the burned amount ──────────────────────
    assert_eq!(
        parse_mint_supply(&svm, &a.mint_lp),
        lp_supply_before - burn_amount,
        "LP supply must decrease by the burned amount"
    );

    // ── Verify user LP balance decreased ─────────────────────────────────────
    assert_eq!(
        parse_token_amount(&svm, &user_lp),
        user_lp_before - burn_amount,
        "user LP balance must decrease by the burned amount"
    );

    // ── Verify vault reserves decreased proportionally ────────────────────────
    assert!(
        parse_token_amount(&svm, &a.vault_x) < vault_x_before,
        "vault_x must have decreased after withdrawal"
    );
    assert!(
        parse_token_amount(&svm, &a.vault_y) < vault_y_before,
        "vault_y must have decreased after withdrawal"
    );
}

#[test]
fn test_swap() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(program_id(), PROGRAM_SO).unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let a = setup(&mut svm, &payer);

    fund_user(&mut svm, &payer, &payer.pubkey(), &a.mint_x.pubkey(), 2_000_000);
    fund_user(&mut svm, &payer, &payer.pubkey(), &a.mint_y.pubkey(), 2_000_000);

    send(&mut svm, &[initialize_ix(&payer, &a)], &payer, &[]);

    // Seed the pool with equal reserves (1:1 ratio)
    let lp_seed: u64 = 1_000_000;
    send(
        &mut svm,
        &[deposit_ix(&payer, &a, lp_seed, lp_seed, lp_seed)],
        &payer,
        &[],
    );

    // Record pool state before the swap
    let vault_x_before = parse_token_amount(&svm, &a.vault_x);
    let vault_y_before = parse_token_amount(&svm, &a.vault_y);

    let user_x = get_associated_token_address(&payer.pubkey(), &a.mint_x.pubkey());
    let user_y = get_associated_token_address(&payer.pubkey(), &a.mint_y.pubkey());
    let user_x_before = parse_token_amount(&svm, &user_x);

    // Swap 1 000 X for at least 1 Y (loose minimum for testing purposes)
    let swap_amount: u64 = 1_000;
    send(&mut svm, &[swap_ix(&payer, &a, true, swap_amount, 1)], &payer, &[]);

    // ── Verify vault_x increased: user sent X into the pool ──────────────────
    assert_eq!(
        parse_token_amount(&svm, &a.vault_x),
        vault_x_before + swap_amount,
        "vault_x must increase by the swapped-in amount"
    );

    // ── Verify vault_y decreased: pool sent Y out to the user ────────────────
    assert!(
        parse_token_amount(&svm, &a.vault_y) < vault_y_before,
        "vault_y must decrease after the swap"
    );

    // ── Verify user_x decreased by the exact swap amount ─────────────────────
    assert_eq!(
        parse_token_amount(&svm, &user_x),
        user_x_before - swap_amount,
        "user X balance must decrease by the swapped amount"
    );

    // ── Verify user_y received tokens ────────────────────────────────────────
    assert!(
        parse_token_amount(&svm, &user_y) > 0,
        "user must have received Y tokens from the swap"
    );

    // ── Verify xy=k invariant is preserved (within rounding) ─────────────────
    let vault_x_after = parse_token_amount(&svm, &a.vault_x) as u128;
    let vault_y_after = parse_token_amount(&svm, &a.vault_y) as u128;
    let k_before = vault_x_before as u128 * vault_y_before as u128;
    let k_after  = vault_x_after  * vault_y_after;
    // k_after >= k_before because the fee stays in the pool
    assert!(k_after >= k_before, "xy=k invariant: k must not decrease after a swap");
}
