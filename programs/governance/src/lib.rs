use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::Sysvar;

// Using your program ID
declare_id!("HbD9TyCRmTboM3QuL2h227hEhzKBfL3CTgqtohtGKP92");

// Import the necessary items from the on-chain program
use on_chain::program::OnChain;
use on_chain::{self, BondAccount};

#[program]
pub mod governance {
    use super::*;

    // No change to initialize_governance
    pub fn initialize_governance(
        ctx: Context<InitializeGovernance>,
        voting_period: i64,
        quorum_votes: u64,
    ) -> Result<()> {
        let governance_state = &mut ctx.accounts.governance_state;
        governance_state.admin = *ctx.accounts.admin.key;
        governance_state.voting_period = voting_period;
        governance_state.quorum_votes = quorum_votes;
        governance_state.proposal_count = 0;
        Ok(())
    }

    /// NEW INSTRUCTION: An admin can add a new verified user.
    pub fn add_verified_user(ctx: Context<AddVerifiedUser>) -> Result<()> {
        let verified_user = &mut ctx.accounts.verified_user;
        verified_user.authority = ctx.accounts.user_to_verify.key();
        verified_user.is_verified = true;
        msg!("User {} has been verified.", verified_user.authority);
        Ok(())
    }

    /// MODIFIED: Now requires the proposer to be verified.
    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        description: String,
        bond_purpose: String,
        bond_sector: String,
        bond_amount: u64,
    ) -> Result<()> {
        let current_slot = Clock::get()?.slot;
        let voting_period_slots = ctx.accounts.governance_state.voting_period as u64;
        let proposal_id = ctx.accounts.governance_state.proposal_count;
        let proposal = &mut ctx.accounts.proposal;
        proposal.id = proposal_id;
        proposal.proposer = *ctx.accounts.proposer.key;
        proposal.description = description;
        proposal.yes_votes = 0;
        proposal.no_votes = 0;
        proposal.state = ProposalState::Voting;
        proposal.start_slot = current_slot;
        proposal.end_slot = current_slot + voting_period_slots;
        proposal.bond_purpose = bond_purpose;
        proposal.bond_sector = bond_sector;
        proposal.bond_amount = bond_amount;
        let governance_state = &mut ctx.accounts.governance_state;
        governance_state.proposal_count += 1;
        msg!("Proposal #{} created by verified user.", proposal.id);
        Ok(())
    }

    // No change to cast_vote, execute_proposal, or create_bond_via_cpi
    pub fn cast_vote(ctx: Context<CastVote>, vote_yes: bool) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        require!(proposal.state == ProposalState::Voting, GovernanceError::ProposalNotActive);
        require!(Clock::get()?.slot <= proposal.end_slot, GovernanceError::VotingPeriodEnded);
        if vote_yes {
            proposal.yes_votes += 1;
        } else {
            proposal.no_votes += 1;
        }
        let vote_record = &mut ctx.accounts.vote_record;
        vote_record.proposal_id = proposal.id;
        vote_record.voter = *ctx.accounts.voter.key;
        msg!("Vote cast on proposal #{}", proposal.id);
        Ok(())
    }

    pub fn execute_proposal(ctx: Context<ExecuteProposal>) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let governance_state = &ctx.accounts.governance_state;
        require!(proposal.state == ProposalState::Voting, GovernanceError::ProposalNotActive);
        require!(Clock::get()?.slot > proposal.end_slot, GovernanceError::VotingPeriodNotOver);
        require!(proposal.yes_votes > proposal.no_votes, GovernanceError::VoteFailed);
        require!(proposal.yes_votes >= governance_state.quorum_votes, GovernanceError::QuorumNotReached);
        proposal.state = ProposalState::Succeeded;
        msg!("Proposal #{} Succeeded. Ready for CPI.", proposal.id);
        Ok(())
    }
    
    pub fn create_bond_via_cpi(ctx: Context<CreateBondViaCpi>) -> Result<()> {
        let proposal = &ctx.accounts.proposal;
        require!(proposal.state == ProposalState::Succeeded, GovernanceError::ProposalNotSucceeded);
        msg!("Executing CPI to initialize_bond...");
        let cpi_program = ctx.accounts.on_chain_program.to_account_info();
        let cpi_accounts = on_chain::cpi::accounts::InitializeBond {
            bond_account: ctx.accounts.new_bond_account.to_account_info(),
            issuer: ctx.accounts.proposer.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        on_chain::cpi::initialize_bond(
            cpi_ctx,
            proposal.bond_purpose.clone(),
            proposal.bond_sector.clone(),
            proposal.bond_amount,
        )?;
        let proposal = &mut ctx.accounts.proposal;
        proposal.state = ProposalState::Executed;
        msg!("CPI successful. New bond created.");
        Ok(())
    }
}

// --- STATE ACCOUNTS ---
#[account]
#[derive(InitSpace)]
pub struct GovernanceState {
    pub admin: Pubkey,
    pub voting_period: i64,
    pub quorum_votes: u64,
    pub proposal_count: u64,
}

/// NEW ACCOUNT to represent a verified user.
#[account]
#[derive(InitSpace)]
pub struct VerifiedUser {
    pub authority: Pubkey,
    pub is_verified: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub enum ProposalState { Voting, Succeeded, Failed, Executed }
#[account]
#[derive(InitSpace)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Pubkey,
    #[max_len(200)]
    pub description: String,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub start_slot: u64,
    pub end_slot: u64,
    pub state: ProposalState,
    #[max_len(100)]
    pub bond_purpose: String,
    #[max_len(50)]
    pub bond_sector: String,
    pub bond_amount: u64,
}
#[account]
#[derive(InitSpace)]
pub struct VoteRecord {
    pub proposal_id: u64,
    pub voter: Pubkey,
}

// --- INSTRUCTION CONTEXTS ---
#[derive(Accounts)]
pub struct InitializeGovernance<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + GovernanceState::INIT_SPACE,
        seeds = [b"governance_state"],
        bump
    )]
    pub governance_state: Account<'info, GovernanceState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

/// NEW CONTEXT for adding a verified user.
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
    /// CHECK: The user we are verifying.
    pub user_to_verify: UncheckedAccount<'info>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

/// MODIFIED CONTEXT to check for verification.
#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub governance_state: Account<'info, GovernanceState>,
    #[account(
        init,
        payer = proposer,
        space = 8 + Proposal::INIT_SPACE,
        seeds = [b"proposal", governance_state.proposal_count.to_le_bytes().as_ref()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    #[account(mut)]
    pub proposer: Signer<'info>,
    // This constraint ensures the proposer has a valid `VerifiedUser` account.
    #[account(
        seeds = [b"verified_user", proposer.key().as_ref()],
        bump,
        constraint = verified_user.is_verified @ GovernanceError::UserNotVerified
    )]
    pub verified_user: Account<'info, VerifiedUser>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CastVote<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    #[account(mut)]
    pub voter: Signer<'info>,
    #[account(constraint = voter_bond_account.authority == voter.key() @ GovernanceError::NotBondHolder)]
    pub voter_bond_account: Account<'info, BondAccount>,
    #[account(
        init,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [b"vote", proposal.id.to_le_bytes().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteProposal<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    pub governance_state: Account<'info, GovernanceState>,
}

#[derive(Accounts)]
pub struct CreateBondViaCpi<'info> {
    #[account(mut, has_one = proposer)]
    pub proposal: Account<'info, Proposal>,
    #[account(mut)]
    pub proposer: Signer<'info>,
    #[account(mut)]
    /// CHECK: This is safe because the on_chain program will initialize it.
    pub new_bond_account: AccountInfo<'info>,
    pub on_chain_program: Program<'info, OnChain>,
    pub system_program: Program<'info, System>,
}

// --- ERRORS ---
#[error_code]
pub enum GovernanceError {
    #[msg("You are not a bond holder and cannot vote.")]
    NotBondHolder,
    #[msg("This proposal is not active for voting.")]
    ProposalNotActive,
    #[msg("The voting period has ended for this proposal.")]
    VotingPeriodEnded,
    #[msg("The voting period is not over yet.")]
    VotingPeriodNotOver,
    #[msg("Proposal did not receive enough yes votes to pass.")]
    VoteFailed,
    #[msg("The minimum quorum of votes was not reached.")]
    QuorumNotReached,
    #[msg("This proposal has not passed the vote yet.")]
    ProposalNotSucceeded,
    #[msg("The user creating the proposal is not verified.")]
    UserNotVerified,
}
