use anchor_lang::prelude::*;

// The unique on-chain address of your program.
declare_id!("6hPPwfMV5yMR6pCvg1kt2JaAT5FSRjnefuYQ74s62XLL");

// Main program module
#[program]
pub mod on_chain {
    use super::*;

    /// Initializes a new bond account with specified details.
    /// This function sets up the initial state of a funding bond.
    pub fn initialize_bond(
        ctx: Context<InitializeBond>,
        purpose: String,
        sector: String,
        amount: u64,
    ) -> Result<()> {
        // Log a message to the transaction logs.
        msg!("Initializing a new bond...");

        // Get a mutable reference to the bond account.
        let bond_account = &mut ctx.accounts.bond_account;

        // Set the properties of the bond account.
        bond_account.authority = *ctx.accounts.issuer.key;
        bond_account.purpose = purpose;
        bond_account.sector = sector;
        bond_account.amount = amount;
        bond_account.is_redeemed = false; // The bond starts as not redeemed.

        msg!("Bond Initialized:");
        msg!(" -> Authority: {}", bond_account.authority);
        msg!(" -> Purpose: {}", bond_account.purpose);
        msg!(" -> Sector: {}", bond_account.sector);
        msg!(" -> Amount: {}", bond_account.amount);

        Ok(())
    }

    /// Transfers the authority of a bond to a new owner.
    /// Only the current authority can perform this action.
    pub fn transfer_bond(ctx: Context<TransferBond>, new_authority: Pubkey) -> Result<()> {
        msg!(
            "Transferring bond from {} to {}",
            ctx.accounts.bond_account.authority,
            new_authority
        );

        // Update the authority field in the bond account to the new authority's public key.
        ctx.accounts.bond_account.authority = new_authority;

        msg!("Transfer complete.");
        Ok(())
    }
}

// 1. DEFINE THE STATE
// This is the main account that holds all the data for a single funding bond.
// It derives Account to be a Solana account, and InitSpace to automatically
// calculate the required space on the blockchain.
#[account]
#[derive(InitSpace)]
pub struct BondAccount {
    /// The public key of the current owner/controller of the bond.
    pub authority: Pubkey,
    /// A description of what the funds are for.
    #[max_len(100)]
    pub purpose: String,
    /// The sector the funding belongs to (e.g., Healthcare, Education).
    #[max_len(50)]
    pub sector: String,
    /// The monetary value or amount of the bond.
    pub amount: u64,
    /// A flag to indicate if the bond has been redeemed.
    pub is_redeemed: bool,
}

// 2. CREATE CORE INSTRUCTIONS (Contexts)
// This defines the accounts required for the `initialize_bond` instruction.
#[derive(Accounts)]
pub struct InitializeBond<'info> {
    // This creates a new account owned by the program.
    // `payer = issuer` means the `issuer` will pay for the account's rent.
    // `space = 8 + BondAccount::INIT_SPACE` allocates the necessary space. 8 bytes are for the discriminator.
    #[account(
        init,
        payer = issuer,
        space = 8 + BondAccount::INIT_SPACE
    )]
    pub bond_account: Account<'info, BondAccount>,

    // The user who is creating the bond. They must sign the transaction.
    #[account(mut)]
    pub issuer: Signer<'info>,

    // A required account for creating new accounts on Solana.
    pub system_program: Program<'info, System>,
}

// This defines the accounts required for the `transfer_bond` instruction.
#[derive(Accounts)]
pub struct TransferBond<'info> {
    // The bond account that is being transferred. We need mutable access to change its data.
    // `has_one = authority` is a security check that ensures the `authority` signer
    // is the actual owner of the `bond_account`.
    #[account(mut, has_one = authority)]
    pub bond_account: Account<'info, BondAccount>,

    // The current owner of the bond. They must sign to approve the transfer.
    pub authority: Signer<'info>,
}
