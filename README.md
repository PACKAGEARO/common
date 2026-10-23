<img src="assets/icon.svg" width="64" height="64" alt="">

# Common Chain

A community grant treasury on Solana. Members stake CMN to join, with one vote per
member. Anyone can fund a pool with SOL or standard SPL tokens. Members vote to
approve a recipient, asset, total spending ceiling and expiration, or to revoke a grant.
The selected setup is **10 CMN to join**, **7 days to vote**, and **20% participation**
plus a simple majority: yes votes must outnumber no votes. The required number of
votes is rounded up and saved when each proposal opens. For 100 members, at least
20 must vote; at least 11 yes votes out of 20 passes. Later joins or departures do
not change that proposal's participation requirement. Each staked wallet has one
vote; this does not establish one unique person per wallet.

## Reserved grants

Executing a passed grant proposal sets aside its **entire ceiling** before activating
the grant. For example, a pool with 10 SOL that approves a 6 SOL grant has only 4 SOL
left for other grants. The recipient can withdraw their reserved funds in multiple
transfers, including payments to vendors, without another vote.

- SOL reserves live in the program-owned grant account, above its account rent.
- SPL reserves live in the associated token account of the grant vault PDA
  `["vault", grant]`. Create this account with the standard associated-token
  instruction before approval, or in the same transaction. Its rent is paid by
  the account creator and remains in the token account after the grant ends.
- Approval fails atomically if the pool cannot fund the ceiling. The proposal
  remains unexecuted and can be retried after funding, before the grant expires.
- Revocation returns unused funds to the original pool in the same transaction.
- Expiration immediately blocks withdrawals. Anyone can call `release_grant` to
  return the unused balance to the original pool; expiration does not send a
  transaction automatically. Repeating release cannot return funds twice.
- Pool vault balances represent unreserved funds; grant account/token balances
  represent reserved funds. Read both for a pool's total holdings. SOL grant
  reserves exclude account rent.

The program checks recipient signatures, pool ownership, token mints, ceilings,
expiry and revocation. Members' stakes remain in a separate stake vault. Deposits,
approvals, withdrawals, revocations and reserve returns emit on-chain events;
plain transfers do not emit deposit events. Grant accounts remain readable after release.

## Program and clients

`programs/common/src/instructions/` contains instruction entry points and logic;
`state/` contains account and proposal types; `constants.rs` owns PDA seeds.
`tests/common.ts` exercises the grant lifecycle. `app/` has no UI yet.
The [reservation workflow](diagrams/2026-10-22-grant-reservations.svg) shows custody
and return paths.

The reservation and percentage-quorum update changes the treasury and proposal
account layouts. It was deployed and initialized on devnet on 2026-10-22;
an older initialized deployment would need migration before upgrading. Clients must use the
regenerated `target/idl/common.json` and `target/types/common.ts`: approval now
requires reserve accounts, withdrawal uses the grant's custody accounts, and
revocation includes the nested `release` account group. See the test helpers for
account derivation and atomic SPL account creation. Executing proposals no longer
needs a treasury account because the quorum is saved in each proposal. The
`initialize` quorum argument is now a percentage from 1 to 100. Devnet was checked
to have no program-owned accounts before deployment.

Token-2022 is not supported. Cross-chain custody is not implemented; adding an
asset would require bridge-specific validation, custody, transfer and return logic.
The upgrade authority can still replace the program.

## Build and test

With Anchor 1.2.1, the Solana toolchain and installed npm dependencies:

```sh
npm ci
NO_DNA=1 anchor build
NO_DNA=1 anchor test --provider.cluster localnet
npx tsc --noEmit
```

The tests run on local Surfpool and use its time-travel RPC. They create local test
tokens and accounts only. Do not run this test suite against devnet.
Deploy sBPFv3 artifacts with Solana CLI 4.3 or newer; older CLI verification can
reject the binary even when the build and local tests pass.

The devnet program is `12Jd4XMpddAdZ4GpnB61zxmGtwMEDKpJneg676v5HHXG`.
The CMN mint is `8h6qWJUqL5DFm8nVmf9Hh7iw7FxZsfpbzrUg1nGVwWZv`, with 6 decimals.
It was created without an initial supply. Minting and distribution remain separate
actions under the existing deploy wallet's mint authority; no freeze authority was enabled.

`migrations/devnet.json` owns the selected mint and treasury settings. After deploying
the program, run `NO_DNA=1 anchor migrate --provider.cluster devnet` to initialize
with 10 CMN, 604800 seconds and 20 percent. The script verifies the devnet genesis
hash, converts the stake using the actual mint decimals, simulates initialization,
and leaves an already matching treasury unchanged. It refuses conflicting settings.
