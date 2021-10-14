use crate::errors::GloweError as Error;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub enum GloweInstruction {
    /// Mints an NFT taking care of creating the necessary accounts (still need to be passed!!)
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of the user minting
    /// 1. `[]` The account that will receive the NFT
    /// 3. `[writable]` The PDA used for minting
    /// 4. `[writable]` The PDA used to store the token
    /// 5. `[]` The token program (SPL)
    /// 6. `[]` The System program
    /// 7. `[]` The Rent sysvar, needed by the token program
    Mint {
        /// Amount of a specific NFT to mint
        name: String,
        url: String,
    },

    /// Mint an NFT
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of the user minting
    /// 1. `[]` The account that will receive the NFT
    /// 3. `[writable]` The account to used for minting, the owner must be the token program
    /// 4. `[writable]` The account to used to store the token, the owner must be the token program
    /// 5. `[]` The token program (SPL)
    /// 6. `[]` The Rent sysvar, needed by the token program
    Mint2 {
        /// Amount of a specific NFT to mint
        name: String,
        url: String,
    },
}

pub(crate) fn derive_mint_account_internal(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    payer: &Pubkey,
    nft_name: &str,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &derive_mint_account_seeds(program_id, token_program_id, payer, nft_name),
        program_id,
    )
}

pub(crate) fn derive_mint_account_seeds<'a>(
    program_id: &'a Pubkey,
    token_program_id: &'a Pubkey,
    payer: &'a Pubkey,
    nft_name: &'a str,
) -> [&'a [u8]; 6] {
    [
        b"glowenft",
        nft_name.as_bytes(),
        b"mint",
        program_id.as_ref(),
        token_program_id.as_ref(),
        payer.as_ref(),
    ]
}

/// Retrieve the mint account
pub fn get_mint_account(minter: &Pubkey, nft_name: &str) -> Pubkey {
    derive_mint_account_internal(
        &Pubkey::new_from_array([42; 32]),
        &spl_token::id(),
        minter,
        nft_name,
    )
    .0
}

pub(crate) fn derive_token_account_internal(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    payer: &Pubkey,
    nft_name: &str,
    owner: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &derive_token_account_seeds(program_id, token_program_id, payer, nft_name, owner),
        program_id,
    )
}

pub(crate) fn derive_token_account_seeds<'a>(
    program_id: &'a Pubkey,
    token_program_id: &'a Pubkey,
    payer: &'a Pubkey,
    nft_name: &'a str,
    owner: &'a Pubkey,
) -> [&'a [u8]; 7] {
    [
        b"glowenft",
        nft_name.as_bytes(),
        b"owner",
        program_id.as_ref(),
        token_program_id.as_ref(),
        payer.as_ref(),
        owner.as_ref(),
    ]
}

/// Retrieve the mint account
pub fn get_token_account(owner: &Pubkey, minter: &Pubkey, nft_name: &str) -> Pubkey {
    derive_token_account_internal(
        &Pubkey::new_from_array([42; 32]),
        &spl_token::id(),
        minter,
        nft_name,
        owner,
    )
    .0
}

/// Create a new `Mint` instruction
///
/// `program_id` should be this program's id
/// `name` is the name of the NFT
/// `url` is the associated URL
/// `payer` is the account that will be signing and paying fees
/// `owner` is the account that will own the minted NFT at the end, usually matches `payer`
pub fn mint(
    program_id: &Pubkey,
    name: &str,
    url: &str,
    payer: &Pubkey,
    owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = GloweInstruction::Mint {
        name: name.to_string(),
        url: url.to_string(),
    };
    let data = data.try_to_vec().expect("serializing instruction failed");

    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new(get_mint_account(payer, name), false),
            AccountMeta::new(get_token_account(owner, payer, name), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),
        ],
        data,
    })
}

/// Create a new `Mint2` instruction
///
/// `program_id` should be this program's id
/// `name` is the name of the NFT
/// `url` is the associated URL
/// `payer` is the account that will be signing and paying fees
/// `owner` is the account that will own the minted NFT at the end, usually matches `payer`
/// `mint` is the account to be used for minting
/// `token_holder` is the account to be used to hold the minted tokens
pub fn mint2(
    program_id: &Pubkey,
    name: &str,
    url: &str,
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    token_holder: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = GloweInstruction::Mint2 {
        name: name.to_string(),
        url: url.to_string(),
    };
    let data = data.try_to_vec().expect("serializing instruction failed");

    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new(*token_holder, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),
        ],
        data,
    })
}
