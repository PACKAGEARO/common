import * as anchor from "@anchor-lang/core";
import { BN, EventParser, Program } from "@anchor-lang/core";
import {
  createMint,
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";
import { Common } from "../target/types/common";

const { Keypair, LAMPORTS_PER_SOL, PublicKey } = anchor.web3;
type PublicKey = anchor.web3.PublicKey;
type Keypair = anchor.web3.Keypair;

describe("common", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.common as Program<Common>;
  const conn = provider.connection;
  // The upgrade authority. It only sets up the treasury; it has no spending power.
  const authority = (provider.wallet as anchor.Wallet).payer;

  const pda = (...seeds: Buffer[]) =>
    PublicKey.findProgramAddressSync(seeds, program.programId)[0];
  const u64 = (n: number) => new BN(n).toArrayLike(Buffer, "le", 8);
  const stakeVault = pda(Buffer.from("stake_vault"));
  const pool = pda(Buffer.from("pool"), u64(0));
  const vault = pda(Buffer.from("vault"), pool.toBuffer());
  const proposalPda = (id: number) => pda(Buffer.from("proposal"), u64(id));
  const grantPda = (proposalId: number) =>
    pda(Buffer.from("grant"), proposalPda(proposalId).toBuffer());
  const grantVault = (id: number) => pda(Buffer.from("vault"), grantPda(id).toBuffer());
  const grantToken = (id: number, mint: PublicKey) => getAssociatedTokenAddressSync(mint, grantVault(id), true);
  const programData = PublicKey.findProgramAddressSync(
    [program.programId.toBuffer()],
    new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111")
  )[0];

  const CMN = 1_000_000; // 6 decimals
  const MIN_STAKE = 10 * CMN;
  const VOTING_PERIOD = 7 * 86_400; // seven days; the tests time-travel past it
  const QUORUM = 20; // percent of members, rounded up
  const SOL = { sol: {} };

  const [alice, bob, carol, sponsor, grantee] = [0, 0, 0, 0, 0].map(() => Keypair.generate());
  const payee = Keypair.generate().publicKey; // a vendor the grantee pays directly
  let cmnMint: PublicKey;
  let cmnOf: Map<Keypair, PublicKey>;
  let vaultCmn: PublicKey;
  let payeeCmn: PublicKey;

  const expectError = async (tx: Promise<unknown>, code: string) => {
    try {
      await tx;
    } catch (e: any) {
      const text = [e.error?.errorCode?.code, e.message, ...(e.logs ?? [])].join(" ");
      expect(text).to.contain(code);
      return;
    }
    throw new Error(`expected transaction to fail with ${code}`);
  };
  const tokenBalance = async (account: PublicKey) =>
    Number((await getAccount(conn, account)).amount);
  const solReserve = async (id: number) => {
    const account = await conn.getAccountInfo(grantPda(id));
    return account.lamports - await conn.getMinimumBalanceForRentExemption(account.data.length);
  };
  const eventsOf = async (sig: string) => {
    const tx = await conn.getTransaction(sig, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    return [...new EventParser(program.programId, program.coder).parseLogs(tx.meta.logMessages)];
  };
  // Local surfpool only advances its clock on new blocks, so the tests jump it forward.
  const travelTo = (unixSeconds: number) =>
    fetch(conn.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [{ absoluteTimestamp: unixSeconds * 1000 }],
      }),
    });
  const votingEndsAt = async (id: number) =>
    (await program.account.proposal.fetch(proposalPda(id))).votingEndsAt.toNumber();

  const deposit = (from: Keypair, amount: number, depositorToken: PublicKey = null, vaultToken: PublicKey = null) =>
    program.methods
      .deposit(new BN(amount))
      .accountsPartial({ depositor: from.publicKey, pool, depositorToken, vaultToken })
      .signers([from])
      .rpc();
  const propose = (id: number, action: any, by = alice) =>
    program.methods
      .createProposal(`proposal ${id}`, action)
      .accountsPartial({ proposer: by.publicKey, proposal: proposalPda(id) })
      .signers([by])
      .rpc();
  const approveGrant = (asset: any, ceiling: number, expiresAt: number) => ({
    approveGrant: {
      pool,
      grantee: grantee.publicKey,
      asset,
      ceiling: new BN(ceiling),
      expiresAt: new BN(expiresAt),
    },
  });
  const vote = (id: number, voter: Keypair, approve: boolean) =>
    program.methods
      .castVote(approve)
      .accountsPartial({ voter: voter.publicKey, proposal: proposalPda(id) })
      .signers([voter])
      .rpc();
  // Create the grant's canonical token account atomically with approval; SOL needs none.
  const executeGrant = (id: number, vaultToken: PublicKey = null, mint: PublicKey = null) =>
    program.methods
      .executeGrantProposal()
      .accountsPartial({ executor: grantee.publicKey, proposal: proposalPda(id), pool, grant: grantPda(id), vaultToken, grantToken: mint ? grantToken(id, mint) : null })
      .preInstructions(mint ? [createAssociatedTokenAccountIdempotentInstruction(grantee.publicKey, grantToken(id, mint), grantVault(id), mint)] : [])
      .signers([grantee])
      .rpc();
  const releaseAccounts = (id: number, mint: PublicKey = null) => ({
    grant: grantPda(id), pool,
    vaultToken: mint ? grantToken(id, mint) : null,
    poolToken: mint ? vaultCmn : null,
  });
  const release = (id: number, mint: PublicKey = null) =>
    program.methods.releaseGrant().accountsPartial(releaseAccounts(id, mint)).rpc();
  const withdraw = (grantId: number, amount: number, destination: PublicKey, vaultToken: PublicKey = null, by = grantee) =>
    program.methods
      .withdrawGrant(new BN(amount))
      .accountsPartial({ grantee: by.publicKey, grant: grantPda(grantId), pool, destination, vaultToken })
      .signers([by])
      .rpc();

  before(async () => {
    for (const kp of [alice, bob, carol, sponsor, grantee]) {
      const sig = await conn.requestAirdrop(kp.publicKey, 10 * LAMPORTS_PER_SOL);
      await conn.confirmTransaction(sig);
    }
    cmnMint = await createMint(conn, authority, authority.publicKey, null, 6);
    cmnOf = new Map();
    for (const kp of [alice, bob, carol, sponsor]) {
      const ata = await getOrCreateAssociatedTokenAccount(conn, authority, cmnMint, kp.publicKey);
      await mintTo(conn, authority, cmnMint, ata.address, authority, 1000 * CMN);
      cmnOf.set(kp, ata.address);
    }
    vaultCmn = (await getOrCreateAssociatedTokenAccount(conn, authority, cmnMint, vault, true)).address;
    payeeCmn = (await getOrCreateAssociatedTokenAccount(conn, authority, cmnMint, payee)).address;
  });

  const initialize = (payer: Keypair, quorum = QUORUM) =>
    program.methods
      .initialize(new BN(MIN_STAKE), new BN(VOTING_PERIOD), quorum)
      .accountsPartial({ payer: payer.publicKey, cmnMint, program: program.programId, programData })
      .signers([payer])
      .rpc();

  it("only the upgrade authority can initialize", async () => {
    await expectError(initialize(alice), "Unauthorized");
    await expectError(initialize(authority, 0), "InvalidConfig");
    await expectError(initialize(authority, 101), "InvalidConfig");
    await initialize(authority);
    const treasury = await program.account.treasury.fetch(pda(Buffer.from("treasury")));
    expect(treasury.minStake.toNumber()).to.equal(10 * CMN);
    expect(treasury.votingPeriod.toNumber()).to.equal(7 * 86_400);
    expect(treasury.quorumPercent).to.equal(20);
  });

  it("members join by staking CMN", async () => {
    for (const kp of [alice, bob, carol]) {
      await program.methods
        .join()
        .accountsPartial({ owner: kp.publicKey, ownerCmn: cmnOf.get(kp) })
        .signers([kp])
        .rpc();
    }
    expect(await tokenBalance(stakeVault)).to.equal(3 * MIN_STAKE);
  });

  it("members open grant pools that anyone can fund, with deposits recorded on-chain", async () => {
    const createPool = (by: Keypair) =>
      program.methods.createPool("Open source tooling").accountsPartial({ creator: by.publicKey, pool }).signers([by]).rpc();
    await expectError(createPool(sponsor), "AccountNotInitialized");
    await createPool(alice);

    const sig = await deposit(sponsor, 3 * LAMPORTS_PER_SOL);
    const [event] = await eventsOf(sig);
    expect(event.name).to.equal("deposited");
    expect(event.data.depositor.toBase58()).to.equal(sponsor.publicKey.toBase58());
    expect(event.data.amount.toNumber()).to.equal(3 * LAMPORTS_PER_SOL);

    await deposit(authority, LAMPORTS_PER_SOL);
    await deposit(sponsor, 500 * CMN, cmnOf.get(sponsor), vaultCmn);
    expect(await conn.getBalance(vault)).to.equal(4 * LAMPORTS_PER_SOL);
    expect(await tokenBalance(vaultCmn)).to.equal(500 * CMN);

    // A "deposit" into an account the vault doesn't own is refused.
    await expectError(deposit(sponsor, 1, cmnOf.get(sponsor), cmnOf.get(alice)), "WrongVault");
  });

  it("the community votes on grants with spending ceilings and expiry dates", async () => {
    // Pin the clock so expiry dates are predictable.
    const t0 = Math.floor(Date.now() / 1000) + 86_400;
    await travelTo(t0);

    await expectError(propose(0, approveGrant(SOL, LAMPORTS_PER_SOL, t0)), "InvalidGrant");

    await propose(0, approveGrant(SOL, LAMPORTS_PER_SOL, t0 + VOTING_PERIOD + 1800)); // passes 2-1
    await propose(1, approveGrant({ spl: { mint: cmnMint } }, 200 * CMN, t0 + 30 * 86_400)); // passes 2-0
    await propose(2, approveGrant(SOL, 2 * LAMPORTS_PER_SOL, t0 + 30 * 86_400)); // rejected 1-2

    await vote(0, alice, true);
    await vote(0, bob, true);
    await vote(0, carol, false);
    await vote(1, alice, true);
    await vote(1, bob, true);
    await vote(2, alice, true);
    await vote(2, bob, false);
    await vote(2, carol, false);

    await expectError(vote(0, alice, true), "already in use");
    await expectError(executeGrant(0), "VotingOpen");

    await travelTo((await votingEndsAt(2)) + 1);
    await executeGrant(0);
    await executeGrant(1, vaultCmn, cmnMint);
    await expectError(executeGrant(2), "ProposalRejected");
    await expectError(executeGrant(0), "already in use");

    const grant = await program.account.grant.fetch(grantPda(0));
    expect(grant.grantee.toBase58()).to.equal(grantee.publicKey.toBase58());
    expect(grant.ceiling.toNumber()).to.equal(LAMPORTS_PER_SOL);
    expect(grant.withdrawn.toNumber()).to.equal(0);
    expect(await conn.getBalance(vault)).to.equal(3 * LAMPORTS_PER_SOL);
    expect(await solReserve(0)).to.equal(LAMPORTS_PER_SOL);
    expect(await tokenBalance(vaultCmn)).to.equal(300 * CMN);
    expect(await tokenBalance(grantToken(1, cmnMint))).to.equal(200 * CMN);
    await expectError(release(0), "GrantActive");
    await expectError(release(1, cmnMint), "GrantActive");
  });

  it("grantees withdraw within their limits without a new vote", async () => {
    await withdraw(0, 0.4 * LAMPORTS_PER_SOL, payee);
    await withdraw(0, 0.4 * LAMPORTS_PER_SOL, payee);
    expect(await conn.getBalance(payee)).to.equal(0.8 * LAMPORTS_PER_SOL);
    expect((await program.account.grant.fetch(grantPda(0))).withdrawn.toNumber()).to.equal(0.8 * LAMPORTS_PER_SOL);
    await expectError(withdraw(0, 0.3 * LAMPORTS_PER_SOL, payee), "CeilingExceeded");
    await expectError(withdraw(0, 1, payee, null, bob), "Unauthorized");

    await expectError(withdraw(1, 1, payeeCmn, vaultCmn), "WrongVault");
    const sig = await withdraw(1, 150 * CMN, payeeCmn, grantToken(1, cmnMint));
    expect(await tokenBalance(payeeCmn)).to.equal(150 * CMN);
    const [event] = await eventsOf(sig);
    expect(event.name).to.equal("grantWithdrawn");
    expect(event.data.totalWithdrawn.toNumber()).to.equal(150 * CMN);

    // Members' stakes are out of reach: the pool vault isn't the stake vault's authority.
    await expectError(withdraw(1, 1, payeeCmn, stakeVault), "WrongVault");
    expect(await tokenBalance(stakeVault)).to.equal(3 * MIN_STAKE);
  });

  it("the community can revoke a grant, and grants expire", async () => {
    await propose(3, { revokeGrant: { grant: grantPda(1) } });
    await vote(3, alice, true);
    await vote(3, bob, true);
    await travelTo((await votingEndsAt(3)) + 1);

    await expectError(executeGrant(3), "WrongProposalType");
    // Returning funds and revoking authority must either both succeed or both roll back.
    await expectError(program.methods.executeRevokeProposal().accountsPartial({
      proposal: proposalPda(3), release: { ...releaseAccounts(1, cmnMint), poolToken: payeeCmn },
    }).rpc(), "WrongVault");
    expect((await program.account.grant.fetch(grantPda(1))).revoked).to.equal(false);
    expect((await program.account.proposal.fetch(proposalPda(3))).executed).to.equal(false);
    await program.methods
      .executeRevokeProposal()
      .accountsPartial({ proposal: proposalPda(3), release: releaseAccounts(1, cmnMint) })
      .rpc();

    expect((await program.account.grant.fetch(grantPda(1))).revoked).to.equal(true);
    expect(await tokenBalance(vaultCmn)).to.equal(350 * CMN);
    expect(await tokenBalance(grantToken(1, cmnMint))).to.equal(0);
    await expectError(withdraw(1, 1, payeeCmn, grantToken(1, cmnMint)), "GrantRevoked");
    await expectError(withdraw(0, 1, payee), "GrantExpired");
    const sig = await release(0);
    const [event] = await eventsOf(sig);
    expect(event.name).to.equal("grantFundsReleased");
    expect(event.data.amount.toNumber()).to.equal(0.2 * LAMPORTS_PER_SOL);
    expect(await conn.getBalance(vault)).to.equal(3.2 * LAMPORTS_PER_SOL);
    await release(0); // A repeat cannot return the same money twice.
    await release(1, cmnMint);
    expect(await conn.getBalance(vault)).to.equal(3.2 * LAMPORTS_PER_SOL);
    expect(await tokenBalance(vaultCmn)).to.equal(350 * CMN);
  });

  it("cannot promise the same SOL or tokens twice; a funded proposal can be retried", async () => {
    const expiry = (await votingEndsAt(3)) + 30 * 86_400;
    await propose(4, approveGrant(SOL, 4 * LAMPORTS_PER_SOL, expiry));
    await propose(5, approveGrant({ spl: { mint: cmnMint } }, 501 * CMN, expiry));
    for (const id of [4, 5]) {
      await vote(id, alice, true);
      await vote(id, bob, true);
    }
    await travelTo((await votingEndsAt(5)) + 1);
    await expectError(executeGrant(4), "InsufficientFunds");
    await expectError(executeGrant(5, vaultCmn, cmnMint), "InsufficientFunds");
    expect((await program.account.proposal.fetch(proposalPda(4))).executed).to.equal(false);
    expect(await conn.getAccountInfo(grantPda(4))).to.equal(null);
    expect(await conn.getAccountInfo(grantToken(5, cmnMint))).to.equal(null);
    await deposit(authority, LAMPORTS_PER_SOL);
    await deposit(sponsor, 200 * CMN, cmnOf.get(sponsor), vaultCmn);
    await executeGrant(4);
    await executeGrant(5, vaultCmn, cmnMint);
    expect(await solReserve(4)).to.equal(4 * LAMPORTS_PER_SOL);
    expect(await tokenBalance(grantToken(5, cmnMint))).to.equal(501 * CMN);
    expect(await conn.getBalance(vault)).to.equal(0.2 * LAMPORTS_PER_SOL);
    expect(await tokenBalance(vaultCmn)).to.equal(49 * CMN);
    await expectError(withdraw(5, 1, payeeCmn, grantToken(1, cmnMint)), "WrongVault");
  });

  it("refuses expired approvals and returns expired token reserves only to their pool", async () => {
    const expiry = (await votingEndsAt(5)) + VOTING_PERIOD + 10;
    await propose(6, approveGrant(SOL, 1, expiry));
    await vote(6, alice, true);
    await vote(6, bob, true);
    await travelTo(expiry);
    await expectError(executeGrant(6), "GrantExpired");
    expect(await conn.getAccountInfo(grantPda(6))).to.equal(null);
    const grant = await program.account.grant.fetch(grantPda(5));
    await travelTo(grant.expiresAt.toNumber());
    await expectError(program.methods.releaseGrant().accountsPartial({ ...releaseAccounts(5, cmnMint), poolToken: payeeCmn }).rpc(), "WrongVault");
    await release(5, cmnMint);
    expect(await tokenBalance(vaultCmn)).to.equal(550 * CMN);
    await release(4);
    expect(await conn.getBalance(vault)).to.equal(4.2 * LAMPORTS_PER_SOL);
  });

  it("supports SOL reserves below account rent and preserves grant history after returning them", async () => {
    const expiry = (await program.account.grant.fetch(grantPda(5))).expiresAt.toNumber() + VOTING_PERIOD + 10;
    await propose(7, approveGrant(SOL, 2, expiry));
    await vote(7, alice, true);
    await vote(7, bob, true);
    await travelTo((await votingEndsAt(7)) + 1);
    await executeGrant(7);
    expect(await solReserve(7)).to.equal(2);
    await withdraw(7, 1, payee);
    expect(await solReserve(7)).to.equal(1);
    await travelTo(expiry);
    await release(7);
    await release(7);
    expect(await solReserve(7)).to.equal(0);
    expect((await program.account.grant.fetch(grantPda(7))).withdrawn.toNumber()).to.equal(1);
  });

  it("snapshots 20% quorum rounded up and requires a simple majority", async () => {
    const expiry = (await program.account.grant.fetch(grantPda(7))).expiresAt.toNumber() + 30 * 86_400;
    await propose(8, approveGrant(SOL, 1, expiry)); // Three members: ceil(3 * 20%) = 1.

    const newcomers = [sponsor, Keypair.generate(), Keypair.generate()];
    for (const kp of newcomers) {
      if (kp !== sponsor) {
        const sig = await conn.requestAirdrop(kp.publicKey, LAMPORTS_PER_SOL);
        await conn.confirmTransaction(sig);
        const ata = await getOrCreateAssociatedTokenAccount(conn, authority, cmnMint, kp.publicKey);
        await mintTo(conn, authority, cmnMint, ata.address, authority, MIN_STAKE);
        cmnOf.set(kp, ata.address);
      }
      await program.methods.join().accountsPartial({ owner: kp.publicKey, ownerCmn: cmnOf.get(kp) }).signers([kp]).rpc();
    }
    expect((await program.account.treasury.fetch(pda(Buffer.from("treasury")))).memberCount).to.equal(6);
    for (const id of [9, 10, 11]) {
      await propose(id, approveGrant(SOL, 1, expiry)); // Six members: ceil(6 * 20%) = 2.
      expect((await program.account.proposal.fetch(proposalPda(id))).quorum).to.equal(2);
    }
    expect((await program.account.proposal.fetch(proposalPda(8))).quorum).to.equal(1);
    await vote(8, alice, true);
    await vote(9, alice, true); // Majority, but not enough participation.
    await vote(10, alice, true);
    await vote(10, bob, false); // Enough participation, but a tie fails.
    await vote(11, alice, true);
    await vote(11, bob, true);

    for (const kp of newcomers) {
      await program.methods.leave().accountsPartial({ owner: kp.publicKey, ownerCmn: cmnOf.get(kp) }).signers([kp]).rpc();
    }
    expect((await program.account.proposal.fetch(proposalPda(9))).quorum).to.equal(2);
    await travelTo((await votingEndsAt(11)) + 1);
    await executeGrant(8);
    await expectError(executeGrant(9), "QuorumNotReached");
    await expectError(executeGrant(10), "ProposalRejected");
    await executeGrant(11);
  });

  it("members get their stake back when they leave", async () => {
    const aliceCmn = cmnOf.get(alice);
    const before = await tokenBalance(aliceCmn);
    await program.methods
      .leave()
      .accountsPartial({ owner: alice.publicKey, ownerCmn: aliceCmn })
      .signers([alice])
      .rpc();
    expect(await tokenBalance(aliceCmn)).to.equal(before + MIN_STAKE);
    expect(await tokenBalance(stakeVault)).to.equal(2 * MIN_STAKE);
    expect((await program.account.treasury.fetch(pda(Buffer.from("treasury")))).memberCount).to.equal(2);
  });
});
