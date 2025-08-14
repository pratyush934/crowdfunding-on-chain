import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OnChain } from "../target/types/on_chain";
import { Keypair, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

describe("on-chain", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.OnChain as Program<OnChain>;
  const provider = anchor.getProvider();

  it("Initializes a new bond!", async () => {
    const bondAccount = Keypair.generate();
    const issuer = Keypair.generate();

    // Airdrop SOL to issuer
    await provider.connection.requestAirdrop(
      issuer.publicKey,
      anchor.web3.LAMPORTS_PER_SOL
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));

    const tx = await program.methods
      .initializeBond(
        "Fund for new school library",
        "Education",
        new anchor.BN(1000)
      )
      .accounts({
        bondAccount: bondAccount.publicKey,
        issuer: issuer.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([issuer, bondAccount])
      .rpc();

    console.log("Initialize transaction signature", tx);

    const account = await program.account.bondAccount.fetch(
      bondAccount.publicKey
    );
    console.log("Bond account created successfully!");
    console.log("Account data:", {
      authority: account.authority,
      purpose: account.purpose,
      sector: account.sector,
      amount: account.amount,
      isRedeemed: account.isRedeemed,
    });

    expect(account.purpose).to.equal("Fund for new school library");
    expect(account.sector).to.equal("Education");
    expect(account.amount.toNumber()).to.equal(1000);
    expect(account.isRedeemed).to.equal(false);
  });

  it("Transfers the bond to a new authority!", async () => {
    const bondAccount = Keypair.generate();
    const issuer = Keypair.generate();
    const newAuthority = Keypair.generate(); // This stays as Keypair to get the pubkey

    // Airdrop SOL
    await provider.connection.requestAirdrop(
      issuer.publicKey,
      anchor.web3.LAMPORTS_PER_SOL
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // Initialize bond
    await program.methods
      .initializeBond(
        "Fund for new school library",
        "Education",
        new anchor.BN(1000)
      )
      .accounts({
        bondAccount: bondAccount.publicKey,
        issuer: issuer.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([issuer, bondAccount])
      .rpc();

    console.log("New authority public key:", newAuthority.publicKey.toBase58());

    // Transfer bond - FIX: Use 'authority' instead of 'currentAuthority'
    const tx = await program.methods
      .transferBond(newAuthority.publicKey)
      .accounts({
        bondAccount: bondAccount.publicKey,
        authority: issuer.publicKey, // This should match your Rust struct field name
      })
      .signers([issuer])
      .rpc();

    console.log("Transfer transaction signature", tx);

    const account = await program.account.bondAccount.fetch(
      bondAccount.publicKey
    );
    console.log("Bond transferred successfully!");
    console.log("Updated account data:", {
      authority: account.authority,
      purpose: account.purpose,
      sector: account.sector,
      amount: account.amount,
      isRedeemed: account.isRedeemed,
    });

    expect(account.authority.toBase58()).to.equal(
      newAuthority.publicKey.toBase58()
    );
  });
});
