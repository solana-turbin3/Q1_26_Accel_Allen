import { createRequire } from "module";
const require = createRequire(import.meta.url);

// Force CJS imports to avoid broken ESM in @helium packages
const anchor = require("@coral-xyz/anchor");
const { PublicKey, Keypair, Connection, ComputeBudgetProgram } = require("@solana/web3.js");
const {
  init: initTuktuk,
  createTaskQueue,
  getTaskQueueForName,
  taskQueueAuthorityKey,
} = require("@helium/tuktuk-sdk");
const { init: initCron, createCronJob } = require("@helium/cron-sdk");

import * as fs from "fs";
import * as path from "path";
import * as crypto from "crypto";

// Program IDs
const PROGRAM_ID = new PublicKey("H8Tq9DAw82BcYzeeBpm3BLisK8sQn4Ntyj3AewhNTuvj");
const ORACLE_PROGRAM_ID = new PublicKey("LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab");

// Derive PDAs
const [gptConfig] = PublicKey.findProgramAddressSync(
  [Buffer.from("gpt_config")],
  PROGRAM_ID
);
const [payerPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("payer")],
  PROGRAM_ID
);

async function main() {
  const rpcUrl = process.env.RPC_URL || "https://api.devnet.solana.com";
  const connection = new Connection(rpcUrl, "confirmed");

  // Load wallet
  let adminKeypair;
  const secretKey = process.env.ADMIN_SECRET_KEY;
  if (secretKey) {
    try {
      adminKeypair = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(secretKey)));
    } catch {
      const bs58 = (await import("bs58")).default;
      adminKeypair = Keypair.fromSecretKey(bs58.decode(secretKey));
    }
  } else {
    const keypath = path.resolve(process.env.HOME, ".config/solana/id.json");
    adminKeypair = Keypair.fromSecretKey(
      Uint8Array.from(JSON.parse(fs.readFileSync(keypath, "utf-8")))
    );
  }

  const wallet = new anchor.Wallet(adminKeypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  const taskQueueName = process.env.TASK_QUEUE_NAME || "solana-gpt-tuktuk";
  const schedule = process.env.CRON_SCHEDULE || "0 */5 * * * * *";

  // Init SDK programs
  const tuktukProgram = await initTuktuk(provider);
  const cronProgram = await initCron(provider);

  console.log("Wallet:", wallet.publicKey.toBase58());
  console.log("GptConfig PDA:", gptConfig.toBase58());
  console.log("Payer PDA:", payerPda.toBase58());

  // Get or create task queue
  let taskQueue = await getTaskQueueForName(tuktukProgram, taskQueueName);
  if (!taskQueue) {
    console.log(`Creating task queue "${taskQueueName}"...`);
    const method = await createTaskQueue(tuktukProgram, {
      name: taskQueueName,
      minCrankReward: new anchor.BN(0),
      capacity: 100,
      lookupTables: [],
      staleTaskAge: new anchor.BN(3600),
    });
    const tx = await method.rpc();
    console.log("  Task queue created, tx:", tx);
    taskQueue = await getTaskQueueForName(tuktukProgram, taskQueueName);
  }
  console.log("Task queue:", taskQueue.toBase58());

  // Fetch GptConfig to get context_account
  const gptConfigInfo = await connection.getAccountInfo(gptConfig);
  if (!gptConfigInfo) {
    throw new Error("GptConfig not initialized. Call initialize first.");
  }
  const contextAccount = new PublicKey(gptConfigInfo.data.subarray(40, 72));
  console.log("Context account:", contextAccount.toBase58());

  // Derive oracle interaction PDA
  const [interaction] = PublicKey.findProgramAddressSync(
    [Buffer.from("interaction"), payerPda.toBuffer(), contextAccount.toBuffer()],
    ORACLE_PROGRAM_ID
  );

  // Build the ask_gpt instruction
  const discriminator = crypto
    .createHash("sha256")
    .update("global:ask_gpt")
    .digest()
    .subarray(0, 8);

  const askGptIx = new anchor.web3.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: gptConfig, isSigner: false, isWritable: false },
      { pubkey: payerPda, isSigner: false, isWritable: true },
      { pubkey: interaction, isSigner: false, isWritable: true },
      { pubkey: contextAccount, isSigner: false, isWritable: false },
      { pubkey: ORACLE_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: anchor.web3.SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(discriminator),
  });

  console.log(`Creating cron job with schedule: ${schedule}`);

  try {
    const method = await createCronJob(cronProgram, {
      tuktukProgram,
      taskQueue,
      args: {
        name: "ask-gpt",
        schedule,
        instructions: [askGptIx],
        freeTasksPerTransaction: 0,
        numTasksPerQueueCall: 1,
      },
    });
    const tx = await method.rpc();
    console.log("Cron job created!");
    console.log("  Transaction:", tx);
  } catch (err) {
    if (err.message?.includes("already in use") || err.logs?.some(l => l.includes("already in use"))) {
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
