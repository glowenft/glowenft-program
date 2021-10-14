#[cfg(feature = "entrypoint")]
mod entrypoint;

pub mod processor;

pub mod errors;

pub mod instructions;

#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;
    use solana_program_test::*;
    use solana_sdk::{signature::Signer, transaction::Transaction};

    use crate::instructions as ixs;

    const NFT_NAME: &str = "GloweNFT";

    #[tokio::test]
    async fn test_minting() {
        let program_id = Pubkey::new_from_array([42; 32]);

        let mut runtime = ProgramTest::default();
        runtime.add_program(
            "glowenft",
            program_id,
            processor!(crate::entrypoint::process_instruction),
        );

        let spl_programs = programs::spl_programs(&solana_program::rent::Rent::default());
        for (spl_program_id, spl_program_data) in spl_programs.into_iter() {
            runtime.add_account(spl_program_id, spl_program_data.into())
        }

        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "glowenft",
            program_id,
            processor!(crate::entrypoint::process_instruction),
        )
        .start()
        .await;

        let body = ixs::mint(
            &program_id,
            NFT_NAME,
            "https://glowenft.com",
            &payer.pubkey(),
            &payer.pubkey(),
            // &mint_pda,
            // &token_account_pda,
        )
        .expect("create Mint transaction");

        let mut transaction = Transaction::new_with_payer(&[body], Some(&payer.pubkey()));
        transaction.sign(&[&payer], recent_blockhash);

        assert!(banks_client.process_transaction(transaction).await.is_ok())
    }
}
