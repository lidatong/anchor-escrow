#![cfg(feature = "test-bpf")]

use std::sync::Arc;

use anchor_client::solana_sdk::{system_program, sysvar};
use anchor_lang::prelude::*;
use anchor_lang::{solana_program, AccountDeserialize, AccountSerialize};
use anchor_spl::token::Token;

use anchor_escrow::Escrow;
use {
    anchor_client::{
        solana_sdk::{
            account::Account, commitment_config::CommitmentConfig, pubkey::Pubkey,
            signature::Keypair, signature::Signer, transaction::Transaction,
        },
        Client, Cluster,
    },
    anchor_escrow,
    solana_program_test::{tokio, ProgramTest},
    std::rc::Rc,
};

#[tokio::test]
async fn init() {
    let mint_account = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let initializer = Keypair::new();
    let temp_token_account = Pubkey::new_unique();
    let token_to_receive_account = Pubkey::new_unique();
    let escrow_account = Keypair::new();

    let mut pt = ProgramTest::new("anchor_escrow", anchor_escrow::id(), None);
    pt.add_account(initializer.pubkey(), Account::default());
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let client = Client::new_with_options(
        Cluster::Debug,
        Keypair::new(),
        CommitmentConfig::processed(),
    );

    // TODO setup incl. minting token, etc.

    let program = client.program(anchor_escrow::id());
    let ix = program
        .request()
        .accounts(
            anchor_escrow::accounts::InitEscrow {
                initializer: initializer.pubkey(),
                temp_token_account,
                token_to_receive_account,
                escrow_account: escrow_account.pubkey(),
                rent: sysvar::rent::id(),
                token_program: anchor_spl::token::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        )
        .args(anchor_escrow::instruction::InitEscrow { amount: 1 })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &initializer, &escrow_account],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();
}
