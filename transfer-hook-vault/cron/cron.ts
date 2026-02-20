import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, Connection } from "@solana/web3.js";
import { cronJobTransaction, taskQueueAuthorityKey } from "@helium/cron-sdk";
import { tuktukKey } from "@helium/tuktuk-sdk";
import "dotenv/config";

// Program IDs
const PROGRAM_ID = new PublicKey("4Uoq2yp6eCji8xx6H7F1SgWWV732TnJhK7rjcyWMp7Fs");
const TUKTUK_PROGRAM_ID = new PublicKey("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");

// Derive PDAs
const [vaultConfig] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault_config")],
  PROGRAM_ID
);
const [queueAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from("queue_authority")],
  PROGRAM_ID
);

async function main() {
  const rpcUrl = process.env.RPC_URL || "http://localhost:8899";
  const connection = new Connection(rpcUrl, "confirmed");

  // Load wallet from env
  const secretKey = process.env.ADMIN_SECRET_KEY;
  if (!secretKey) {
    throw new Error("ADMIN_SECRET_KEY env var required (base58 or JSON array)");
  }

  let adminKeypair: Keypair;
  try {
    adminKeypair = Keypair.fromSecretKey(
      Uint8Array.from(JSON.parse(secretKey))
    );
  } catch {
    const bs58 = await import("bs58");
    adminKeypair = Keypair.fromSecretKey(bs58.default.decode(secretKey));
  }

  const wallet = new anchor.Wallet(adminKeypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  const taskQueueName = process.env.TASK_QUEUE_NAME || "transfer-hook-vault";

  // Fetch or derive task queue
  const [tuktukConfig] = tuktukKey(TUKTUK_PROGRAM_ID);
  console.log("Tuktuk config:", tuktukConfig.toBase58());
  console.log("Queue authority:", queueAuthority.toBase58());
  console.log("Vault config:", vaultConfig.toBase58());

  // Build the apply_merkle_root_update instruction
  // Anchor discriminator: sha256("global:apply_merkle_root_update")[..8]
  const crypto = await import("crypto");
  const discriminator = crypto
    .createHash("sha256")
    .update("global:apply_merkle_root_update")
    .digest()
    .subarray(0, 8);

  const applyIx = new anchor.web3.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: vaultConfig, isSigner: false, isWritable: true },
    ],
    data: Buffer.from(discriminator),
  });

  // Create a cron job that calls apply_merkle_root_update every minute
  const schedule = process.env.CRON_SCHEDULE || "*/1 * * * *"; // every minute
  console.log(`Creating cron job with schedule: ${schedule}`);

  try {
    const { transaction, taskQueue, cronJob } = await cronJobTransaction({
      provider,
      cronJobName: "apply-merkle-root",
      schedule,
      instructions: [applyIx],
      taskQueueName,
      tuktukProgramId: TUKTUK_PROGRAM_ID,
    });

    const sig = await provider.sendAndConfirm(transaction);
    console.log("Cron job created!");
    console.log("  Transaction:", sig);
    console.log("  Task queue:", taskQueue.toBase58());
    console.log("  Cron job:", cronJob.toBase58());
  } catch (err: any) {
    if (err.message?.includes("already in use")) {
      console.log("Cron job already exists. Skipping creation.");
    } else {
      throw err;
    }
  }
}

main().catch((err) => {
  console.error("Error:", err);
  process.exit(1);
});
