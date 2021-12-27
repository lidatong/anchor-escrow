import * as anchor from '@project-serum/anchor';
import {Program} from '@project-serum/anchor';
import {AnchorEscrow} from '../target/types/anchor_escrow';
import {AccountLayout, Token, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction
} from "@solana/web3.js";
import * as assert from "assert";

describe('anchorEscrow', () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.AnchorEscrow as Program<AnchorEscrow>;
  const connection = new Connection("http://localhost:8899", 'singleGossip');

  let admin;
  let tokenX: Token;
  let tokenY: Token;
  let initializer: Keypair;
  let initializerTokensX: PublicKey;
  let initializerTokensY: PublicKey;
  let taker: Keypair;
  let takerTokensX: PublicKey;
  let takerTokensY: PublicKey;
  let tempTokenAccount: Keypair;
  let escrowAccount: Keypair;

  const amountX = 42;
  const amountY = 24;

  it('setup', async () => {
    admin = anchor.web3.Keypair.generate();
    await connection.confirmTransaction(await connection.requestAirdrop(
        admin.publicKey,
        LAMPORTS_PER_SOL
    ));

    // Set up Token Mint
    tokenX = await Token.createMint(
        connection,
        admin,
        admin.publicKey,
        null,
        0,
        TOKEN_PROGRAM_ID
    );
    tokenY = await Token.createMint(
        connection,
        admin,
        admin.publicKey,
        null,
        0,
        TOKEN_PROGRAM_ID
    );

    // Alice
    initializer = anchor.web3.Keypair.generate();
    await connection.confirmTransaction(await connection.requestAirdrop(
        initializer.publicKey,
        LAMPORTS_PER_SOL
    ));
    initializerTokensX = await tokenX.createAccount(initializer.publicKey);
    await tokenX.mintTo(initializerTokensX, admin, [], 100);
    initializerTokensY = await tokenY.createAccount(initializer.publicKey);

    // Bob
    taker = anchor.web3.Keypair.generate();
    await connection.confirmTransaction(await connection.requestAirdrop(
        taker.publicKey,
        LAMPORTS_PER_SOL
    ));
    takerTokensX = await tokenX.createAccount(taker.publicKey);
    takerTokensY = await tokenY.createAccount(taker.publicKey);
    await tokenY.mintTo(takerTokensY, admin, [], 100);

    // setting up input accounts for escrow
    // includes setting up Alice's "tempTokenAccount" as described in the tutorial
    // it's kind of pointless in a unit test, but i'm doing it just for completeness
    tempTokenAccount = anchor.web3.Keypair.generate();
    escrowAccount = anchor.web3.Keypair.generate();
    const XTokenMintAccountPubkey = new PublicKey((await connection.getParsedAccountInfo(initializerTokensX, 'singleGossip')).value!.data.parsed.info.mint);
    const createTempTokenAccountIx = SystemProgram.createAccount({
      programId: TOKEN_PROGRAM_ID,
      space: AccountLayout.span,
      lamports: await connection.getMinimumBalanceForRentExemption(AccountLayout.span, 'singleGossip'),
      fromPubkey: initializer.publicKey,
      newAccountPubkey: tempTokenAccount.publicKey
    });
    const initTempAccountIx = Token.createInitAccountInstruction(TOKEN_PROGRAM_ID, XTokenMintAccountPubkey, tempTokenAccount.publicKey, initializer.publicKey);
    const transferXTokensToTempAccIx = Token.createTransferInstruction(
        TOKEN_PROGRAM_ID, initializerTokensX, tempTokenAccount.publicKey, initializer.publicKey, [], amountX);
    const tx = new Transaction().add(createTempTokenAccountIx, initTempAccountIx, transferXTokensToTempAccIx);
    await connection.confirmTransaction(
        await connection.sendTransaction(tx, [initializer, tempTokenAccount], {
          skipPreflight: false,
          preflightCommitment: 'singleGossip'
        })
    );
  })

  it('initEscrow', async () => {
    await program.rpc.initEscrow(
        new anchor.BN(amountY),
        {
          accounts: {
            initializer: initializer.publicKey,
            tempTokenAccount: tempTokenAccount.publicKey,
            tokenToReceiveAccount: initializerTokensY,
            escrowAccount: escrowAccount.publicKey,
            rent: SYSVAR_RENT_PUBKEY,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId
          },
          signers: [initializer, escrowAccount]
        })
    console.log("owner", (await connection.getAccountInfo(initializer.publicKey)).owner.toString())
    assert.equal(await getBalance(tokenX, initializerTokensX), 100 - amountX);
    assert.equal(await getBalance(tokenX, tempTokenAccount.publicKey), amountX);
    assert.equal((await program.account.escrow.fetch(escrowAccount.publicKey)).expectedAmount.toNumber(), amountY);
  })

  it('exchange', async () => {
    const [pda, _] = await PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode("escrow"))],
        program.programId
    );
    const tx = await program.rpc.exchange(
        new anchor.BN(amountY),
        {
          accounts: {
            taker: taker.publicKey,
            takersSendingTokenAccount: takerTokensY,
            takersTokenToReceiveAccount: takerTokensX,
            pdasTempTokenAccount: tempTokenAccount.publicKey,
            initializer: initializer.publicKey,
            initializersTokenToReceiveAccount: initializerTokensY,
            escrowAccount: escrowAccount.publicKey,
            pdaAccount: pda,
            tokenProgram: TOKEN_PROGRAM_ID
          },
          signers: [taker]
        }
    );
    await connection.confirmTransaction(tx);
    assert.equal(await getBalance(tokenX, initializerTokensX), 100 - amountX);
    assert.equal(await getBalance(tokenY, initializerTokensY), amountY);
    assert.equal(await getBalance(tokenX, takerTokensX), amountX);
    assert.equal(await getBalance(tokenY, takerTokensY), 100 - amountY);
  });
});

const getBalance = async (token: Token, publicKey: PublicKey) => (await token.getAccountInfo(publicKey)).amount.toNumber()