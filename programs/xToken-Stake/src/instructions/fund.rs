use crate::{
    constant::CALC_PRECISION,
    state::{Vault, VaultStatus},
    util::get_now_timestamp,
};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct Fund<'info> {
    // funder
    #[account(signer,
    constraint = funder.key() == vault.authority || vault.funders.iter().any(|x| *x == funder.key())
    )]
    funder: AccountInfo<'info>,

    // authority
    authority: AccountInfo<'info>,

    // vault
    #[account(mut, has_one = authority,
    constraint = vault.status == VaultStatus::Initialized,
    constraint = vault.reward_mint_account == * reward_account.to_account_info().key)]
    vault: Account<'info, Vault>,

    // reward account
    #[account(mut)]
    reward_account: Box<Account<'info, TokenAccount>>,

    // funder account
    #[account(mut,
    constraint = funder_account.amount >= amount)]
    funder_account: Box<Account<'info, TokenAccount>>,

    // token program
    #[account(address = spl_token::id())]
    token_program: AccountInfo<'info>,
}

pub fn fund(ctx: Context<Fund>, amount: u64) -> ProgramResult {
    let vault = &mut ctx.accounts.vault;
    let current_number = vault.stake_token_count;
    let now = get_now_timestamp();

    if now >= vault.reward_duration_deadline {
        vault.reward_rate = (amount as u128)
            .checked_mul(CALC_PRECISION)
            .unwrap()
            .checked_div(vault.reward_duration as u128)
            .unwrap()
            .checked_div(current_number as u128)
            .unwrap();

        vault.reward_duration_deadline = now.checked_add(vault.reward_duration).unwrap();

        msg!("New reward deadline has been set");
    } else {
        let remaining = vault.reward_duration_deadline.checked_sub(now).unwrap();
        let current_value = (vault.reward_rate as u128)
            .checked_mul(remaining as u128)
            .unwrap()
            .checked_mul(current_number as u128)
            .unwrap();

        msg!("Current reward overall amount is {}", current_value);

        vault.reward_rate = (amount as u128)
            .checked_mul(CALC_PRECISION)
            .unwrap()
            .checked_add(current_value)
            .unwrap()
            .checked_div(vault.reward_duration as u128)
            .unwrap()
            .checked_div(current_number as u128)
            .unwrap();

        msg!("New reward rate is {}", vault.reward_rate);
    }

    // transfer token
    let cpi_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        anchor_spl::token::Transfer {
            from: ctx.accounts.funder_account.to_account_info(),
            to: ctx.accounts.reward_account.to_account_info(),
            authority: ctx.accounts.funder.to_account_info(),
        },
    );

    anchor_spl::token::transfer(cpi_context, amount)?;

    Ok(())
}
