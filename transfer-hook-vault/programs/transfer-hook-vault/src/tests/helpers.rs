use {
    anchor_lang::{
        prelude::msg,
        InstructionData,
        ToAccountMetas,
    },
    litesvm::LiteSVM,
    sha2::{Sha256, Digest},
    solana_instruction::Instruction,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_pubkey::Pubkey,
    solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
    solana_signer::Signer,
    solana_transaction::Transaction,
    spl_token_2022::{
        self,
        extension::StateWithExtensions,
        state::Account as TokenAccountState,
    },
    spl_associated_token_account,
    std::path::PathBuf,
};

pub static PROGRAM_ID: Pubkey = Pubkey::from_str_const("4Uoq2yp6eCji8xx6H7F1SgWWV732TnJhK7rjcyWMp7Fs");
pub static TOKEN_2022_PROGRAM_ID: Pubkey = Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

// ===================== Pubkey Bridging =====================

pub fn to_anchor_pubkey(pk: &Pubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(pk.to_bytes())
}

pub fn convert_account_metas(metas: Vec<anchor_lang::prelude::AccountMeta>) -> Vec<solana_instruction::AccountMeta> {
    metas
        .into_iter()
        .map(|m| solana_instruction::AccountMeta {
            pubkey: Pubkey::from(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect()
}

// ===================== Merkle Tree =====================

pub fn hash_leaf(pubkey: &Pubkey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(pubkey.as_ref());
    hasher.finalize().into()
}

fn hash_pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    if a <= b {
        hasher.update(a);
        hasher.update(b);
    } else {
        hasher.update(b);
        hasher.update(a);
    }
    hasher.finalize().into()
}

pub struct MerkleTree {
    layers: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    pub fn new(mut leaves: Vec<[u8; 32]>) -> Self {
        let next_pow2 = leaves.len().next_power_of_two();
        while leaves.len() < next_pow2 {
            leaves.push([0u8; 32]);
        }

        let mut layers = vec![leaves.clone()];
        let mut current = leaves;

        while current.len() > 1 {
            let mut next_layer = Vec::new();
            for chunk in current.chunks(2) {
                next_layer.push(hash_pair(&chunk[0], &chunk[1]));
            }
            layers.push(next_layer.clone());
            current = next_layer;
        }

        MerkleTree { layers }
    }

    pub fn root(&self) -> [u8; 32] {
        self.layers.last().unwrap()[0]
    }

    pub fn proof(&self, index: usize) -> Vec<[u8; 32]> {
        let mut proof = Vec::new();
        let mut idx = index;

        for layer in &self.layers[..self.layers.len() - 1] {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            if sibling_idx < layer.len() {
                proof.push(layer[sibling_idx]);
            }
            idx /= 2;
        }

        proof
    }
}

// ===================== PDA Derivations =====================

pub fn get_vault_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault_config"], &PROGRAM_ID)
}

pub fn get_approval_pda(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"approval", user.as_ref()], &PROGRAM_ID)
}

pub fn get_extra_account_meta_list_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"extra-account-metas", mint.as_ref()],
        &PROGRAM_ID,
    )
}

pub fn get_vault_ata(vault_config: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::from(
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &to_anchor_pubkey(vault_config),
            &to_anchor_pubkey(mint),
            &to_anchor_pubkey(&TOKEN_2022_PROGRAM_ID),
        ).to_bytes()
    )
}

pub fn get_user_ata(user: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::from(
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &to_anchor_pubkey(user),
            &to_anchor_pubkey(mint),
            &to_anchor_pubkey(&TOKEN_2022_PROGRAM_ID),
        ).to_bytes()
    )
}

// ===================== SVM Setup =====================

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let admin = Keypair::new();

    svm.airdrop(&admin.pubkey(), 100 * LAMPORTS_PER_SOL)
        .expect("Failed to airdrop");

    let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/transfer_hook_vault.so");

    let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
    let _ = svm.add_program(PROGRAM_ID, &program_data);

    (svm, admin)
}

/// Full setup: initialize vault, init extra account meta, create 4 users with ATAs and tokens.
pub fn full_setup(
    svm: &mut LiteSVM,
    admin: &Keypair,
) -> (Pubkey, MerkleTree, Vec<Keypair>) {
    let users: Vec<Keypair> = (0..4).map(|_| {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    }).collect();

    let leaves: Vec<[u8; 32]> = users
        .iter()
        .map(|u| hash_leaf(&u.pubkey()))
        .collect();

    let tree = MerkleTree::new(leaves);

    let mint = Keypair::new();
    do_initialize(svm, admin, &mint, tree.root(), 1_000_000);
    let mint_pubkey = mint.pubkey();

    do_init_extra_account_meta(svm, admin, &mint_pubkey);

    for user in &users {
        let ata = create_user_ata(svm, admin, &mint_pubkey, &user.pubkey());
        mint_tokens(svm, admin, &mint_pubkey, &ata, 10_000);
    }

    (mint_pubkey, tree, users)
}

// ===================== Instruction Builders =====================

pub fn do_initialize(
    svm: &mut LiteSVM,
    admin: &Keypair,
    mint: &Keypair,
    merkle_root: [u8; 32],
    initial_supply: u64,
) -> Pubkey {
    let (vault_config_pda, _) = get_vault_config_pda();
    let (vault_approval_pda, _) = get_approval_pda(&vault_config_pda);
    let vault_ata = get_vault_ata(&vault_config_pda, &mint.pubkey());

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: crate::accounts::Initialize {
            admin: to_anchor_pubkey(&admin.pubkey()),
            vault_config: to_anchor_pubkey(&vault_config_pda),
            vault_approval: to_anchor_pubkey(&vault_approval_pda),
            mint: to_anchor_pubkey(&mint.pubkey()),
            vault: to_anchor_pubkey(&vault_ata),
            associated_token_program: to_anchor_pubkey(&Pubkey::from(spl_associated_token_account::ID.to_bytes())),
            token_program: to_anchor_pubkey(&TOKEN_2022_PROGRAM_ID),
            rent: to_anchor_pubkey(&Pubkey::from(solana_sdk_ids::sysvar::rent::ID.to_bytes())),
            system_program: to_anchor_pubkey(&SYSTEM_PROGRAM_ID),
        }
        .to_account_metas(None)
        .into_iter()
        .map(|m| solana_instruction::AccountMeta {
            pubkey: Pubkey::from(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect(),
        data: crate::instruction::Initialize {
            merkle_root,
            initial_supply,
        }
        .data(),
    };

    let msg = Message::new(&[ix], Some(&admin.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[admin, mint], msg, blockhash);
    let result = svm.send_transaction(tx);
    match &result {
        Err(e) => {
            msg!("Initialize failed: {:?}", e.meta.logs);
            panic!("Initialize failed");
        }
        Ok(r) => {
            msg!("Initialize CUs: {}", r.compute_units_consumed);
        }
    }

    vault_config_pda
}

pub fn do_init_extra_account_meta(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
) {
    let (extra_account_meta_list, _) = get_extra_account_meta_list_pda(mint);

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: convert_account_metas(
            crate::accounts::InitializeExtraAccountMetaList {
                payer: to_anchor_pubkey(&payer.pubkey()),
                extra_account_meta_list: to_anchor_pubkey(&extra_account_meta_list),
                mint: to_anchor_pubkey(mint),
                system_program: to_anchor_pubkey(&SYSTEM_PROGRAM_ID),
            }
            .to_account_metas(None),
        ),
        data: crate::instruction::InitializeExtraAccountMetaList {}.data(),
    };

    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[payer], msg, blockhash);
    let result = svm.send_transaction(tx);
    match &result {
        Err(e) => {
            msg!("InitExtraAccountMeta failed: {:?}", e.meta.logs);
            panic!("InitExtraAccountMeta failed");
        }
        Ok(_) => {}
    }
}

pub fn do_create_user_state(
    svm: &mut LiteSVM,
    user: &Keypair,
    proof: Vec<[u8; 32]>,
) -> Result<(), String> {
    let (vault_config_pda, _) = get_vault_config_pda();
    let (approval_pda, _) = get_approval_pda(&user.pubkey());

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: convert_account_metas(
            crate::accounts::CreateUserState {
                user: to_anchor_pubkey(&user.pubkey()),
                vault_config: to_anchor_pubkey(&vault_config_pda),
                approval: to_anchor_pubkey(&approval_pda),
                system_program: to_anchor_pubkey(&SYSTEM_PROGRAM_ID),
            }
            .to_account_metas(None),
        ),
        data: crate::instruction::CreateUserState {
            proof,
        }
        .data(),
    };

    let msg = Message::new(&[ix], Some(&user.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[user], msg, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| {
        e.meta.logs.join("\n")
    })
}

pub fn create_user_ata(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let ata = get_user_ata(owner, mint);

    let spl_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &to_anchor_pubkey(&payer.pubkey()),
        &to_anchor_pubkey(owner),
        &to_anchor_pubkey(mint),
        &to_anchor_pubkey(&TOKEN_2022_PROGRAM_ID),
    );

    let ix = Instruction {
        program_id: Pubkey::from(spl_ix.program_id.to_bytes()),
        accounts: spl_ix.accounts.into_iter().map(|m| solana_instruction::AccountMeta {
            pubkey: Pubkey::from(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        }).collect(),
        data: spl_ix.data,
    };

    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[payer], msg, blockhash);
    svm.send_transaction(tx).expect("Failed to create ATA");

    ata
}

pub fn mint_tokens(
    svm: &mut LiteSVM,
    authority: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    amount: u64,
) {
    let spl_ix = spl_token_2022::instruction::mint_to(
        &to_anchor_pubkey(&TOKEN_2022_PROGRAM_ID),
        &to_anchor_pubkey(mint),
        &to_anchor_pubkey(destination),
        &to_anchor_pubkey(&authority.pubkey()),
        &[],
        amount,
    )
    .unwrap();

    let ix = Instruction {
        program_id: Pubkey::from(spl_ix.program_id.to_bytes()),
        accounts: spl_ix.accounts.into_iter().map(|m| solana_instruction::AccountMeta {
            pubkey: Pubkey::from(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        }).collect(),
        data: spl_ix.data,
    };

    let msg = Message::new(&[ix], Some(&authority.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[authority], msg, blockhash);
    svm.send_transaction(tx).expect("Failed to mint tokens");
}

pub fn build_transfer_checked_ix(
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let (approval_pda, _) = get_approval_pda(authority);
    let (extra_meta_list, _) = get_extra_account_meta_list_pda(mint);

    let spl_ix = spl_token_2022::instruction::transfer_checked(
        &to_anchor_pubkey(&TOKEN_2022_PROGRAM_ID),
        &to_anchor_pubkey(source),
        &to_anchor_pubkey(mint),
        &to_anchor_pubkey(destination),
        &to_anchor_pubkey(authority),
        &[],
        amount,
        6, // decimals
    )
    .unwrap();

    let mut ix = Instruction {
        program_id: Pubkey::from(spl_ix.program_id.to_bytes()),
        accounts: spl_ix.accounts.into_iter().map(|m| solana_instruction::AccountMeta {
            pubkey: Pubkey::from(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        }).collect(),
        data: spl_ix.data,
    };

    ix.accounts.push(solana_instruction::AccountMeta::new_readonly(approval_pda, false));
    ix.accounts.push(solana_instruction::AccountMeta::new_readonly(extra_meta_list, false));
    ix.accounts.push(solana_instruction::AccountMeta::new_readonly(PROGRAM_ID, false));

    ix
}

pub fn do_deposit(
    svm: &mut LiteSVM,
    user: &Keypair,
    mint: &Pubkey,
    amount: u64,
) -> Result<(), String> {
    let (vault_config_pda, _) = get_vault_config_pda();
    let (approval_pda, _) = get_approval_pda(&user.pubkey());
    let vault = get_vault_ata(&vault_config_pda, mint);
    let user_ata = get_user_ata(&user.pubkey(), mint);

    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: convert_account_metas(
            crate::accounts::Deposit {
                user: to_anchor_pubkey(&user.pubkey()),
                vault_config: to_anchor_pubkey(&vault_config_pda),
                approval: to_anchor_pubkey(&approval_pda),
            }
            .to_account_metas(None),
        ),
        data: crate::instruction::Deposit { amount }.data(),
    };

    let transfer_ix = build_transfer_checked_ix(
        &user_ata, mint, &vault, &user.pubkey(), amount,
    );

    let msg = Message::new(&[deposit_ix, transfer_ix], Some(&user.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[user], msg, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| {
        e.meta.logs.join("\n")
    })
}

pub fn do_withdraw(
    svm: &mut LiteSVM,
    user: &Keypair,
    mint: &Pubkey,
    amount: u64,
) -> Result<(), String> {
    let (vault_config_pda, _) = get_vault_config_pda();
    let (approval_pda, _) = get_approval_pda(&user.pubkey());
    let vault = get_vault_ata(&vault_config_pda, mint);
    let user_ata = get_user_ata(&user.pubkey(), mint);

    let withdraw_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: convert_account_metas(
            crate::accounts::Withdraw {
                user: to_anchor_pubkey(&user.pubkey()),
                vault_config: to_anchor_pubkey(&vault_config_pda),
                approval: to_anchor_pubkey(&approval_pda),
                vault: to_anchor_pubkey(&vault),
                token_program: to_anchor_pubkey(&TOKEN_2022_PROGRAM_ID),
            }
            .to_account_metas(None),
        ),
        data: crate::instruction::Withdraw { amount }.data(),
    };

    let transfer_ix = build_transfer_checked_ix(
        &vault, mint, &user_ata, &user.pubkey(), amount,
    );

    let msg = Message::new(&[withdraw_ix, transfer_ix], Some(&user.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(&[user], msg, blockhash);
    svm.send_transaction(tx).map(|_| ()).map_err(|e| {
        e.meta.logs.join("\n")
    })
}

pub fn get_token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let acct = svm.get_account(account).expect("Token account not found");
    let state = StateWithExtensions::<TokenAccountState>::unpack(&acct.data)
        .expect("Failed to unpack token account");
    state.base.amount
}
