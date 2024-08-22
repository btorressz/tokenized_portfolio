use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_lang::InstructionData;
use anchor_spl::token;
use solana_program::pubkey::Pubkey;
use solana_program::instruction::Instruction;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use std::str::FromStr;
use tokenized_portfolio::{self, Portfolio};

// Helper function to set up the program test
fn setup_program_test() -> ProgramTest {
    let program_id = Pubkey::from_str("6SdCjCTYtGeAzcKquDFAm5C5pMEayJhtmxWvmPmp1BXP").unwrap();

    // ProgramTest to instantiate the program
    ProgramTest::new(
        "tokenized_portfolio", // Name of the program
        program_id,
        None, 
    )
}

#[tokio::test]
async fn test_initialize_portfolio() {
    let program_test = setup_program_test();
    let owner = Keypair::new();
    let portfolio_account = Keypair::new();

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create portfolio initialization instruction
    let init_portfolio_ix = Instruction {
        program_id: tokenized_portfolio::ID,
        accounts: tokenized_portfolio::accounts::InitializePortfolio {
            portfolio: portfolio_account.pubkey(),
            owner: owner.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: tokenized_portfolio::instruction::InitializePortfolio {}.data(),
    };

    let mut tx = Transaction::new_with_payer(&[init_portfolio_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &portfolio_account], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Fetch and verify portfolio state
    let portfolio_data = banks_client
        .get_account(portfolio_account.pubkey())
        .await
        .expect("Portfolio account not found")
        .expect("Portfolio account has no data");

    let portfolio_state: Portfolio = Portfolio::try_deserialize(&mut &portfolio_data.data[..]).unwrap();
    assert_eq!(portfolio_state.owner, owner.pubkey());
}

#[tokio::test]
async fn test_add_asset() {
    let program_test = setup_program_test();
    let owner = Keypair::new();
    let portfolio_account = Keypair::new();

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Initialize the portfolio first
    let init_portfolio_ix = Instruction {
        program_id: tokenized_portfolio::ID,
        accounts: tokenized_portfolio::accounts::InitializePortfolio {
            portfolio: portfolio_account.pubkey(),
            owner: owner.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: tokenized_portfolio::instruction::InitializePortfolio {}.data(),
    };

    let mut tx = Transaction::new_with_payer(&[init_portfolio_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &portfolio_account], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Add an asset to the portfolio
    let add_asset_ix = Instruction {
        program_id: tokenized_portfolio::ID,
        accounts: tokenized_portfolio::accounts::AddAsset {
            portfolio: portfolio_account.pubkey(),
            owner: owner.pubkey(),
        }
        .to_account_metas(None),
        data: tokenized_portfolio::instruction::AddAsset {
            asset_symbol: "SOL".to_string(),
            asset_amount: 100,
            asset_value: 1_000_000,
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[add_asset_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Fetch and verify portfolio state
    let portfolio_data = banks_client
        .get_account(portfolio_account.pubkey())
        .await
        .expect("Portfolio account not found")
        .expect("Portfolio account has no data");

    let portfolio_state: Portfolio = Portfolio::try_deserialize(&mut &portfolio_data.data[..]).unwrap();
    assert_eq!(portfolio_state.assets.len(), 1);
    assert_eq!(portfolio_state.assets[0].symbol, "SOL");
    assert_eq!(portfolio_state.assets[0].amount, 100);
}

#[tokio::test]
async fn test_transfer_asset() {
    let program_test = setup_program_test();
    let owner = Keypair::new();
    let portfolio_account = Keypair::new();
    let token_account = Keypair::new();
    let destination_account = Keypair::new();

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Initialize the portfolio first
    let init_portfolio_ix = Instruction {
        program_id: tokenized_portfolio::ID,
        accounts: tokenized_portfolio::accounts::InitializePortfolio {
            portfolio: portfolio_account.pubkey(),
            owner: owner.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: tokenized_portfolio::instruction::InitializePortfolio {}.data(),
    };

    let mut tx = Transaction::new_with_payer(&[init_portfolio_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &portfolio_account], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Add an asset to the portfolio
    let add_asset_ix = Instruction {
        program_id: tokenized_portfolio::ID,
        accounts: tokenized_portfolio::accounts::AddAsset {
            portfolio: portfolio_account.pubkey(),
            owner: owner.pubkey(),
        }
        .to_account_metas(None),
        data: tokenized_portfolio::instruction::AddAsset {
            asset_symbol: "SOL".to_string(),
            asset_amount: 100,
            asset_value: 1_000_000,
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[add_asset_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Transfer asset from the portfolio to the destination account
    let transfer_asset_ix = Instruction {
        program_id: tokenized_portfolio::ID,
        accounts: tokenized_portfolio::accounts::TransferAsset {
            portfolio: portfolio_account.pubkey(),
            token_account: token_account.pubkey(),
            destination_account: destination_account.pubkey(),
            owner: owner.pubkey(),
            token_program: token::ID,
        }
        .to_account_metas(None),
        data: tokenized_portfolio::instruction::TransferAsset {
            asset_symbol: "SOL".to_string(),
            amount: 50,
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[transfer_asset_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Fetch and verify portfolio state
    let portfolio_data = banks_client
        .get_account(portfolio_account.pubkey())
        .await
        .expect("Portfolio account not found")
        .expect("Portfolio account has no data");

    let portfolio_state: Portfolio = Portfolio::try_deserialize(&mut &portfolio_data.data[..]).unwrap();
    assert_eq!(portfolio_state.assets[0].amount, 50); // 100 - 50 = 50
}
