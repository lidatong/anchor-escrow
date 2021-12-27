use anchor_lang::prelude::*;
use anchor_lang::solana_program::msg;
use anchor_lang::solana_program::program::invoke;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token;
use anchor_spl::token::accessor::amount;
use anchor_spl::token::{Token, TokenAccount};
use spl_token::instruction::AuthorityType;
use thiserror::Error;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod anchor_escrow {
    use super::*;

    pub fn init_escrow(ctx: Context<InitEscrow>, amount: u64) -> ProgramResult {
        let initializer = &ctx.accounts.initializer;

        let temp_token_account = &ctx.accounts.temp_token_account;

        let token_to_receive_account = &ctx.accounts.token_to_receive_account;

        let escrow_account = &mut ctx.accounts.escrow_account;

        let rent = &ctx.accounts.rent;
        if !rent.is_exempt(
            escrow_account.to_account_info().lamports(),
            escrow_account.to_account_info().data_len(),
        ) {
            return Err(EscrowError::NotRentExempt.into());
        }

        escrow_account.initializer_pubkey = initializer.key();
        escrow_account.temp_token_account_pubkey = temp_token_account.key();
        escrow_account.initializer_token_to_receive_account_pubkey = token_to_receive_account.key();
        escrow_account.expected_amount = amount;

        msg!("Calling the token program to transfer token account ownership...");
        let (pda, bump_seed) = Pubkey::find_program_address(&[b"escrow"], ctx.program_id);
        msg!("PDA {:?}", pda);
        token::set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::SetAuthority {
                    current_authority: initializer.to_account_info(),
                    account_or_mint: temp_token_account.to_account_info(),
                },
            ),
            AuthorityType::AccountOwner,
            Some(pda),
        )
    }

    pub fn exchange(ctx: Context<Exchange>, amount_expected_by_taker: u64) -> ProgramResult {
        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        msg!(
            "{:?}",
            ctx.accounts
                .takers_sending_token_account
                .to_account_info()
                .key
        );
        msg!(
            "{:?}",
            ctx.accounts
                .initializers_token_to_receive_account
                .to_account_info()
                .key
        );
        msg!("{:?}", ctx.accounts.taker.to_account_info().key);

        msg!("{:?}", ctx.accounts.escrow_account.expected_amount);
        msg!("{:?}", ctx.accounts.pdas_temp_token_account.amount);

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.takers_sending_token_account.to_account_info(),
                    to: ctx
                        .accounts
                        .initializers_token_to_receive_account
                        .to_account_info(),
                    authority: ctx.accounts.taker.to_account_info(),
                },
            ),
            ctx.accounts.escrow_account.expected_amount,
        )?;
        let (_, bump_seed) = Pubkey::find_program_address(&[b"escrow"], ctx.program_id);
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.pdas_temp_token_account.to_account_info(),
                    to: ctx
                        .accounts
                        .takers_token_to_receive_account
                        .to_account_info(),
                    authority: ctx.accounts.pda_account.clone(),
                },
                &[&[&b"escrow"[..], &[bump_seed]]],
            ),
            ctx.accounts.pdas_temp_token_account.amount,
        )
    }
}

#[derive(Accounts)]
pub struct InitEscrow<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(mut)]
    pub temp_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_to_receive_account: Account<'info, TokenAccount>,
    #[account(init, payer = initializer)]
    pub escrow_account: Account<'info, Escrow>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Exchange<'info> {
    pub taker: Signer<'info>,
    #[account(mut)]
    pub takers_sending_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub takers_token_to_receive_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pdas_temp_token_account: Account<'info, TokenAccount>,
    pub initializer: AccountInfo<'info>,
    #[account(mut)]
    pub initializers_token_to_receive_account: Account<'info, TokenAccount>,
    #[account(
    mut,
    constraint = escrow_account.temp_token_account_pubkey == pdas_temp_token_account.key(),
    constraint = escrow_account.initializer_pubkey == initializer.key(),
    constraint = escrow_account.initializer_token_to_receive_account_pubkey == initializers_token_to_receive_account.key(),
    // constraint = escrow_account.expected_amount == pdas_temp_token_account.amount,
    )]
    pub escrow_account: Account<'info, Escrow>,
    pub pda_account: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct Escrow {
    pub initializer_pubkey: Pubkey,
    pub temp_token_account_pubkey: Pubkey,
    pub initializer_token_to_receive_account_pubkey: Pubkey,
    pub expected_amount: u64,
}

#[derive(Error, Debug, Copy, Clone)]
pub enum EscrowError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("Not Rent Exempt")]
    NotRentExempt,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
