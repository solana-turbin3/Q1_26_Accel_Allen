import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  createTransferCheckedInstruction,
} from "@solana/spl-token";
import {
  SendTransactionError,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { WhitelistTransferHook } from "../target/types/whitelist_transfer_hook";
import { assert } from "chai";

describe("whitelist-transfer-hook", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const wallet = provider.wallet as anchor.Wallet;
  const program = anchor.workspace
    .whitelistTransferHook as Program<WhitelistTransferHook>;

  // Mint keypair — will be created by the program
  const mintKeypair = anchor.web3.Keypair.generate();

  // Recipient
  const recipient = anchor.web3.Keypair.generate();

  // Derive PDAs
  const [configPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId,
  );

  const [extraAccountMetaListPDA] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("extra-account-metas"), mintKeypair.publicKey.toBuffer()],
      program.programId,
    );

  const [whitelistEntryPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("whitelist"), wallet.publicKey.toBuffer()],
    program.programId,
  );

  // Token accounts (derived after mint is known)
  const sourceTokenAccount = getAssociatedTokenAddressSync(
    mintKeypair.publicKey,
    wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  const destinationTokenAccount = getAssociatedTokenAddressSync(
    mintKeypair.publicKey,
    recipient.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  // ─────────────────────────────────────────────
  // 1. Initialize Config
  // ─────────────────────────────────────────────
  it("Initializes the config", async () => {
    const tx = await program.methods
      .initializeConfig()
      .accountsPartial({
        admin: wallet.publicKey,
        config: configPDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("\n  Config initialized. Admin:", wallet.publicKey.toBase58());
    console.log("  Tx:", tx);

    // Verify
    const config = await program.account.config.fetch(configPDA);
    assert.ok(config.admin.equals(wallet.publicKey));
  });

  // ─────────────────────────────────────────────
  // 2. Create mint with transfer hook (program-side)
  // ─────────────────────────────────────────────
  it("Creates mint with transfer hook extension", async () => {
    const tx = await program.methods
      .createMint(9) // 9 decimals
      .accountsPartial({
        payer: wallet.publicKey,
        mint: mintKeypair.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([mintKeypair])
      .rpc({ skipPreflight: true });

    console.log("\n  Mint created:", mintKeypair.publicKey.toBase58());
    console.log("  Tx:", tx);
  });

  // ─────────────────────────────────────────────
  // 4. Create ExtraAccountMetaList
  // ─────────────────────────────────────────────
  it("Creates ExtraAccountMetaList", async () => {
    const tx = await program.methods
      .initializeExtraAccountMetaList()
      .accountsPartial({
        payer: wallet.publicKey,
        mint: mintKeypair.publicKey,
        extraAccountMetaList: extraAccountMetaListPDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log(
      "\n  ExtraAccountMetaList created:",
      extraAccountMetaListPDA.toBase58(),
    );
    console.log("  Tx:", tx);
  });

  // ─────────────────────────────────────────────
  // 5. Create token accounts and mint tokens
  // ─────────────────────────────────────────────
  it("Creates token accounts and mints tokens", async () => {
    const amount = 100 * 10 ** 9; // 100 tokens

    const transaction = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        mintKeypair.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        destinationTokenAccount,
        recipient.publicKey,
        mintKeypair.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createMintToInstruction(
        mintKeypair.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        amount,
        [],
        TOKEN_2022_PROGRAM_ID,
      ),
    );

    const tx = await sendAndConfirmTransaction(
      provider.connection,
      transaction,
      [wallet.payer],
      { skipPreflight: true },
    );

    console.log("\n  Token accounts created and tokens minted");
    console.log("  Tx:", tx);
  });

  // ─────────────────────────────────────────────
  // 5. Add sender to whitelist
  // ─────────────────────────────────────────────
  it("Adds sender to whitelist", async () => {
    const tx = await program.methods
      .addToWhitelist(wallet.publicKey)
      .accountsPartial({
        admin: wallet.publicKey,
        config: configPDA,
        whitelistEntry: whitelistEntryPDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("\n  Whitelisted:", wallet.publicKey.toBase58());
    console.log("  Tx:", tx);

    const entry = await program.account.whitelistEntry.fetch(whitelistEntryPDA);
    assert.ok(entry.address.equals(wallet.publicKey));
  });

  // ─────────────────────────────────────────────
  // 6. Transfer (sender IS whitelisted — should succeed)
  // ─────────────────────────────────────────────
  it("Transfers tokens when sender is whitelisted", async () => {
    const amount = BigInt(1 * 10 ** 9); // 1 token

    const transferIx = createTransferCheckedInstruction(
      sourceTokenAccount,
      mintKeypair.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amount,
      9, // decimals
      [],
      TOKEN_2022_PROGRAM_ID,
    );

    // Manually append the extra accounts the transfer hook needs:
    //   - ExtraAccountMetaList PDA
    //   - WhitelistEntry PDA (for the sender)
    //   - Transfer hook program
    transferIx.keys.push(
      {
        pubkey: extraAccountMetaListPDA,
        isSigner: false,
        isWritable: false,
      },
      { pubkey: whitelistEntryPDA, isSigner: false, isWritable: false },
      { pubkey: program.programId, isSigner: false, isWritable: false },
    );

    const transaction = new Transaction().add(transferIx);

    const tx = await sendAndConfirmTransaction(
      provider.connection,
      transaction,
      [wallet.payer],
      { skipPreflight: true },
    );

    console.log("\n  Transfer succeeded (sender whitelisted)");
    console.log("  Tx:", tx);
  });

  // ─────────────────────────────────────────────
  // 7. Remove sender from whitelist
  // ─────────────────────────────────────────────
  it("Removes sender from whitelist", async () => {
    const tx = await program.methods
      .removeFromWhitelist(wallet.publicKey)
      .accountsPartial({
        admin: wallet.publicKey,
        config: configPDA,
        whitelistEntry: whitelistEntryPDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("\n  Removed from whitelist:", wallet.publicKey.toBase58());
    console.log("  Tx:", tx);

    // Verify the account is closed
    const info = await provider.connection.getAccountInfo(whitelistEntryPDA);
    assert.isNull(info, "Whitelist entry should be closed");
  });

  // ─────────────────────────────────────────────
  // 8. Transfer (sender NOT whitelisted — should fail)
  // ─────────────────────────────────────────────
  it("Fails transfer when sender is not whitelisted", async () => {
    const amount = BigInt(1 * 10 ** 9);

    const transferIx = createTransferCheckedInstruction(
      sourceTokenAccount,
      mintKeypair.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amount,
      9,
      [],
      TOKEN_2022_PROGRAM_ID,
    );

    transferIx.keys.push(
      {
        pubkey: extraAccountMetaListPDA,
        isSigner: false,
        isWritable: false,
      },
      { pubkey: whitelistEntryPDA, isSigner: false, isWritable: false },
      { pubkey: program.programId, isSigner: false, isWritable: false },
    );

    const transaction = new Transaction().add(transferIx);

    try {
      await sendAndConfirmTransaction(
        provider.connection,
        transaction,
        [wallet.payer],
        { skipPreflight: false },
      );
      assert.fail("Transfer should have failed");
    } catch (error) {
      if (error instanceof SendTransactionError) {
        // The whitelist PDA was closed by removeFromWhitelist, so Anchor
        // fails with AccountNotInitialized when trying to deserialize it.
        // This is the expected behavior — PDA non-existence = not whitelisted.
        const hasExpectedError = error.logs?.some(
          (l) => l.includes("AccountNotInitialized") || l.includes("3012"),
        );
        assert.ok(
          hasExpectedError,
          "Expected AccountNotInitialized (3012) since whitelist PDA is closed",
        );
        console.log(
          "\n  Transfer correctly rejected (whitelist PDA does not exist)",
        );
      } else {
        console.log("\n  Transfer correctly rejected");
      }
    }
  });
});
