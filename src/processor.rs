use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use crate::{errors::GloweError as Error, instructions::GloweInstruction};

use borsh::BorshDeserialize;

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = GloweInstruction::try_from_slice(instruction_data)
            .map_err(|_| Error::InvalidInstruction)?;

        match instruction {
            GloweInstruction::Mint { name, url } => {
                msg!("Instruction: Mint");
                Self::process_mint(accounts, name, url, program_id)
            }
            GloweInstruction::Mint2 { name, url } => {
                msg!("Instruction: Mint2");
                Self::process_mint2(accounts, name, url, program_id)
            }
        }
    }

    //goes from minter + spl_token + received to full NFT...
    // creates 2 accounts in the process
    fn process_mint(
        accounts: &[AccountInfo],
        name: String,
        url: String,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        //The account to pay and sign the minting
        let minter = next_account_info(account_info_iter)?;
        if !minter.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        //The final recipient of the NFT
        let owner = next_account_info(account_info_iter)?;

        //This is the mint account that will be minting the NFT
        let mint_account_info = next_account_info(account_info_iter)?;

        //account that will hold the nft
        let token_account_info = next_account_info(account_info_iter)?;

        //retrieve SPL Token Program account
        let token_program = next_account_info(account_info_iter)?;
        //check it's SPL token program
        if !spl_token::check_id(token_program.key) {
            return Err(Error::AccountMismatch.into());
        }

        //retrieve System Program account
        let system_program = next_account_info(account_info_iter)?;
        //check it's the system program
        if !solana_program::system_program::check_id(system_program.key) {
            return Err(Error::AccountMismatch.into());
        }

        //verify that the mint account matches the PDA for this NFT
        let (mint_pda, mint_pda_bump_seed) = crate::instructions::derive_mint_account_internal(
            program_id,
            token_program.key,
            minter.key,
            name.as_str(),
        );
        if &mint_pda != mint_account_info.key {
            return Err(Error::AccountMismatch.into());
        }

        //verify that the token account matches the PDA for this NFT
        let (token_account_pda, token_account_pda_bump_seed) =
            crate::instructions::derive_token_account_internal(
                program_id,
                token_program.key,
                minter.key,
                name.as_str(),
                owner.key,
            );
        if &token_account_pda != token_account_info.key {
            return Err(Error::AccountMismatch.into());
        }

        // create mint_seeds (for invoke_signed)
        let mint_seeds_partial = &crate::instructions::derive_mint_account_seeds(
            program_id,
            &token_program.key,
            minter.key,
            name.as_str(),
        )[..];

        let mut mint_seeds = [&[] as &_; 7];
        mint_seeds[..6].copy_from_slice(&mint_seeds_partial[..]);

        let mint_pda_bump_seed = [mint_pda_bump_seed];
        mint_seeds[6] = &mint_pda_bump_seed[..];

        // create token_account_seeds (for invoke_signed)
        let token_account_seeds_partial = &crate::instructions::derive_token_account_seeds(
            program_id,
            &token_program.key,
            minter.key,
            name.as_str(),
            owner.key,
        )[..];

        let mut token_account_seeds = [&[] as &_; 8];
        token_account_seeds[..7].copy_from_slice(&token_account_seeds_partial[..]);

        let token_account_pda_bump_seed = [token_account_pda_bump_seed];
        token_account_seeds[7] = &token_account_pda_bump_seed[..];

        //get Rent sysvar to calculate rent stuff
        let rent_account = next_account_info(account_info_iter)?;
        let rent = Rent::from_account_info(&rent_account)?;

        //CREATE MINT ACCOUNT
        {
            let mint_create_account_ix = solana_program::system_instruction::create_account(
                minter.key,
                &mint_pda,
                rent.minimum_balance(spl_token::state::Mint::LEN),
                spl_token::state::Mint::LEN as u64,
                token_program.key,
            );

            msg!("Calling the system program to create the mint account...");
            invoke_signed(
                &mint_create_account_ix,
                &[
                    minter.clone(),
                    mint_account_info.clone(),
                    token_program.clone(),
                    system_program.clone(),
                ],
                &[&mint_seeds],
            )?;
        }

        //CREATE TOKEN ACCOUNT
        {
            let create_token_account_ix = solana_program::system_instruction::create_account(
                minter.key,
                &token_account_pda,
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                token_program.key,
            );

            msg!("Calling the system program to create the token account...");
            invoke_signed(
                &create_token_account_ix,
                &[
                    minter.clone(),
                    token_account_info.clone(),
                    token_program.clone(),
                    system_program.clone(),
                ],
                &[&token_account_seeds],
            )?;
        }

        //INITIALIZE MINT ACCOUNT
        {
            let initialize_mint_ix = spl_token::instruction::initialize_mint(
                token_program.key,
                &mint_pda,
                &mint_pda,
                None,
                0,
            )?;

            msg!("Calling the token program to initialize the minting account...");
            invoke(
                &initialize_mint_ix,
                &[
                    //mint account
                    mint_account_info.clone(),
                    rent_account.clone(),
                    //token program
                    token_program.clone(),
                ],
            )?;
        }

        //INITIALIZE TOKEN ACCOUNT
        {
            let initialize_token_account_ix = spl_token::instruction::initialize_account(
                token_program.key,
                &token_account_pda,
                &mint_pda,
                owner.key,
            )?;

            msg!("Calling the token program to initialize the token account...");
            invoke(
                &initialize_token_account_ix,
                &[
                    //account to initialize
                    token_account_info.clone(),
                    //mint account
                    mint_account_info.clone(),
                    //the account owner
                    owner.clone(),
                    rent_account.clone(),
                    //token program
                    token_program.clone(),
                ],
            )?;
        }

        //MINT TO TOKEN ACCOUNT
        {
            let mint_to_ix = spl_token::instruction::mint_to(
                token_program.key,
                &mint_pda,
                &token_account_pda,
                &mint_pda, //mint authority
                &[&mint_pda],
                1,
            )?;

            msg!("Calling the token program to mint the NFT to the token account...");
            invoke_signed(
                &mint_to_ix,
                &[
                    mint_account_info.clone(),
                    token_account_info.clone(),
                    //minting authority
                    mint_account_info.clone(),
                    token_program.clone(),
                ],
                &[&mint_seeds[..]],
            )?;
        }

        //REVOKE MINT AUTHORITY
        {
            let remove_mint_authority_ix = spl_token::instruction::set_authority(
                token_program.key,
                &mint_pda,
                None,
                spl_token::instruction::AuthorityType::MintTokens,
                &mint_pda,
                &[&mint_pda],
            )?;

            msg!("Calling the token program to revoke the mint authority...");
            invoke_signed(
                &remove_mint_authority_ix,
                &[
                    mint_account_info.clone(),
                    //minting authority
                    mint_account_info.clone(),
                    token_program.clone(),
                ],
                &[&mint_seeds[..]],
            )?;
        }

        Ok(())
    }

    //same as above, except the 2 accounts are already created
    fn process_mint2(
        accounts: &[AccountInfo],
        name: String,
        url: String,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        //account to mint the token and pay all the fees
        let minter = next_account_info(account_info_iter)?;
        if !minter.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        //account to receive the token
        let receiver = next_account_info(account_info_iter)?;

        //account to mint the token
        let mint = next_account_info(account_info_iter)?;
        //account to hold the token
        let token_account = next_account_info(account_info_iter)?;

        //SPL token program
        let token_program = next_account_info(account_info_iter)?;
        if !spl_token::check_id(token_program.key) {
            return Err(Error::AccountMismatch.into());
        }
        //check that `mint` and `token_account` are SPL token accounts
        if mint.owner != token_program.key || token_account.owner != token_program.key {
            return Err(ProgramError::IllegalOwner);
        }

        //get Rent sysvar
        let rent_account = next_account_info(account_info_iter)?;

        {
            let initialize_mint_ix = spl_token::instruction::initialize_mint(
                token_program.key,
                mint.key,
                //set the minting authority to the minter, temporary
                // as we will see later we remove the authority
                minter.key,
                None,
                0,
            )?;

            msg!("Calling the token program to initialize the minting account...");
            invoke(
                &initialize_mint_ix,
                &[
                    //mint account
                    mint.clone(),
                    rent_account.clone(),
                    //token program
                    token_program.clone(),
                ],
            )?;
        }

        {
            let initialize_token_account_ix = spl_token::instruction::initialize_account(
                token_program.key,
                token_account.key,
                mint.key,
                receiver.key,
            )?;

            msg!("Calling the token program to initialize the token account...");
            invoke(
                &initialize_token_account_ix,
                &[
                    //account to initialize
                    token_account.clone(),
                    //mint account
                    mint.clone(),
                    //the account owner
                    receiver.clone(),
                    rent_account.clone(),
                    //token program
                    token_program.clone(),
                ],
            )?;
        }

        {
            let mint_to_ix = spl_token::instruction::mint_to(
                token_program.key,
                mint.key,
                token_account.key,
                minter.key,
                &[minter.key],
                1,
            )?;

            msg!("Calling the token program to mint the NFT to the token account...");
            invoke(
                &mint_to_ix,
                &[
                    mint.clone(),
                    token_account.clone(),
                    //minting authority
                    minter.clone(),
                ],
            )?;
        }

        {
            let remove_mint_authority_ix = spl_token::instruction::set_authority(
                token_program.key,
                mint.key,
                None,
                spl_token::instruction::AuthorityType::MintTokens,
                minter.key,
                &[minter.key],
            )?;

            msg!("Calling the token program to revoke the mint authority...");
            invoke(
                &remove_mint_authority_ix,
                &[
                    mint.clone(),
                    //minting authority
                    minter.clone(),
                ],
            )?;
        }

        Ok(())
    }
}
