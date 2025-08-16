use anchor_lang::prelude::*;


declare_id!("6ongwoyXhZ119UcadKiAMyJf8adB7J9JwVpUNDD7hD5G");

#[program]
pub mod on_chain {
    use super::*;

    pub fn initialize_bond(
        ctx: Context<InitializeBond>,
        purpose: String,
        sector: String,
        amount: u64,
    ) -> Result<()> {
        msg!("Initializing a new bond...");
        let bond_account = &mut ctx.accounts.bond_account;
        bond_account.authority = *ctx.accounts.issuer.key;
        bond_account.purpose = purpose;
        bond_account.sector = sector;
        bond_account.amount = amount;
        bond_account.is_redeemed = false;
        msg!("Bond Initialized:");
        msg!(" -> Authority: {}", bond_account.authority);
        msg!(" -> Purpose: {}", bond_account.purpose);
        msg!(" -> Sector: {}", bond_account.sector);
        msg!(" -> Amount: {}", bond_account.amount);
        Ok(())
    }

    pub fn transfer_bond(ctx: Context<TransferBond>, new_authority: Pubkey) -> Result<()> {
        msg!(
            "Transferring bond from {} to {}",
            ctx.accounts.bond_account.authority,
            new_authority
        );
        ctx.accounts.bond_account.authority = new_authority;
        msg!("Transfer complete.");
        Ok(())
    }

    pub fn add_verified_user(ctx: Context<AddVerifiedUser>) -> Result<()> {
        msg!(
            "Adding verified user: {}",
            ctx.accounts.user_to_verify.key()
        );
        let verified_user = &mut ctx.accounts.verified_user;
        verified_user.user_pubkey = ctx.accounts.user_to_verify.key();
        verified_user.is_verified = true;
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct BondAccount {
    pub authority: Pubkey,
    #[max_len(100)]
    pub purpose: String,
    #[max_len(50)]
    pub sector: String,
    pub amount: u64,
    pub is_redeemed: bool,
}

#[account]
#[derive(InitSpace)]
pub struct VerifiedUser {
    pub user_pubkey: Pubkey,
    pub is_verified: bool,
}

#[derive(Accounts)]
pub struct InitializeBond<'info> {
    #[account(
        init,
        payer = issuer,
        space = 8 + BondAccount::INIT_SPACE,
        seeds = [b"bond", issuer.key().as_ref()],
        bump
    )]
    pub bond_account: Account<'info, BondAccount>,
    #[account(mut)]
    pub issuer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferBond<'info> {
    #[account(mut, has_one = authority)]
    pub bond_account: Account<'info, BondAccount>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddVerifiedUser<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + VerifiedUser::INIT_SPACE,
        seeds = [b"verified_user", user_to_verify.key().as_ref()],
        bump
    )]
    pub verified_user: Account<'info, VerifiedUser>,
    /// CHECK: This account is being used to create a verified user record
    pub user_to_verify: UncheckedAccount<'info>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}
