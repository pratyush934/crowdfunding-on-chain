import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OnChain } from "../target/types/on_chain";
import { Governance } from "../target/types/governance";
import { assert } from "chai";

// Helper function to sleep for a specified number of milliseconds
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe("governance", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Get references to both of our on-chain programs
  const onChainProgram = anchor.workspace.OnChain as Program<OnChain>;
  const governanceProgram = anchor.workspace.Governance as Program<Governance>;

  // Use a predictable Program Derived Address (PDA) for the governance state
  const [governanceStatePDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("governance_state")],
      governanceProgram.programId
  );

  // Use the provider's wallet as the main actor for simplicity
  const admin = provider.wallet;
  const proposer = provider.wallet;
  const voter = provider.wallet;

  // Keypairs for accounts needed in the test
  const voterBondAccount = anchor.web3.Keypair.generate();
  let proposalKey: anchor.web3.PublicKey;
  let proposalCount: anchor.BN;

  // `before` hook runs once before all tests in this suite
  before(async () => {
    // 1. Create a bond for our voter, giving them voting rights.
    await onChainProgram.methods
      .initializeBond("Test bond for voter", "Community", new anchor.BN(1))
      .accounts({
        bondAccount: voterBondAccount.publicKey,
        issuer: voter.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([voterBondAccount])
      .rpc();
    console.log("Voter's bond account created.");

    // 2. Initialize the main governance state using a PDA.
    const votingPeriod = new anchor.BN(5); // 5 slots = ~2 seconds
    const quorumVotes = new anchor.BN(1); // 1 "yes" vote is enough to pass
    await governanceProgram.methods
      .initializeGovernance(votingPeriod, quorumVotes)
      .accounts({
        governanceState: governanceStatePDA,
        admin: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("Governance state initialized.");
  });

  it("Verifies a new user", async () => {
    // Derive the PDA for the verified user account
    const [verifiedUserPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("verified_user"), proposer.publicKey.toBuffer()],
      governanceProgram.programId
    );

    // The admin calls the instruction to verify the proposer
    await governanceProgram.methods
      .addVerifiedUser()
      .accounts({
        verifiedUser: verifiedUserPDA,
        userToVerify: proposer.publicKey,
        admin: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    
    const verifiedUserAccount = await governanceProgram.account.verifiedUser.fetch(verifiedUserPDA);
    assert.isTrue(verifiedUserAccount.isVerified);
    console.log(`User ${proposer.publicKey.toBase58()} has been verified.`);
  });

  it("Creates a new proposal with a verified user", async () => {
    const govState = await governanceProgram.account.governanceState.fetch(
      governanceStatePDA
    );
    proposalCount = govState.proposalCount;

    // Derive the PDA for the new proposal account
    [proposalKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("proposal"), proposalCount.toBuffer("le", 8)],
      governanceProgram.programId
    );

    // Derive the PDA for the verified user to pass to the instruction
    const [verifiedUserPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("verified_user"), proposer.publicKey.toBuffer()],
      governanceProgram.programId
    );

    await governanceProgram.methods
      .createProposal(
        "Proposal to fund a new public park",
        "Public Park Construction",
        "Infrastructure",
        new anchor.BN(50000)
      )
      .accounts({
        governanceState: governanceStatePDA,
        proposal: proposalKey,
        proposer: proposer.publicKey,
        verifiedUser: verifiedUserPDA, // Pass the verified user account
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("Proposal created successfully by verified user.");
  });

  it("Allows a bond holder to cast a vote", async () => {
    // Derive the PDA for the vote record to prevent double voting
    const [voteRecordKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vote"),
        proposalCount.toBuffer("le", 8),
        voter.publicKey.toBuffer(),
      ],
      governanceProgram.programId
    );

    // Call the cast_vote instruction with a "yes" vote
    await governanceProgram.methods
      .castVote(true)
      .accounts({
        proposal: proposalKey,
        voter: voter.publicKey,
        voterBondAccount: voterBondAccount.publicKey,
        voteRecord: voteRecordKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("Vote cast successfully.");
  });

  it("Successfully executes the proposal in two steps", async () => {
    console.log("Waiting for voting period to end...");
    await sleep(3000); // 3 seconds should be enough for 5 slots on a local validator

    // --- STEP 1: Execute the proposal to change its state ---
    await governanceProgram.methods
      .executeProposal()
      .accounts({
        proposal: proposalKey,
        governanceState: governanceStatePDA,
      })
      .rpc();

    let proposalAccount = await governanceProgram.account.proposal.fetch(
      proposalKey
    );
    assert.equal(proposalAccount.state.hasOwnProperty("succeeded"), true, "Proposal should be in Succeeded state");
    console.log("Step 1 complete: Proposal state is now 'Succeeded'.");

    // --- STEP 2: Call the new instruction to perform the CPI ---
    const newBondAccount = anchor.web3.Keypair.generate();
    await governanceProgram.methods
      .createBondViaCpi()
      .accounts({
        proposal: proposalKey,
        proposer: proposer.publicKey,
        newBondAccount: newBondAccount.publicKey,
        onChainProgram: onChainProgram.programId,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([newBondAccount])
      .rpc();
    
    console.log("Step 2 complete: CPI instruction sent.");

    // --- VERIFICATION ---
    const createdBond = await onChainProgram.account.bondAccount.fetch(
      newBondAccount.publicKey
    );
    assert.ok(createdBond.authority.equals(proposer.publicKey), "Bond authority should be the proposer");
    
    proposalAccount = await governanceProgram.account.proposal.fetch(
      proposalKey
    );
    assert.equal(proposalAccount.state.hasOwnProperty("executed"), true, "Proposal should be in Executed state");

    console.log("Successfully executed proposal and created new bond!");
  });
});