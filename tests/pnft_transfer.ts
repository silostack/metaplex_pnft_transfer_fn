import * as anchor from "@project-serum/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  TOKEN_PROGRAM_ID
} from "@solana/spl-token";

import { Keypair, Transaction } from "@solana/web3.js";
import { expect } from "chai";
import {
  buildAndSendTx,
  createAndFundATA,
  createFundedWallet,
  createTokenAuthorizationRules,
  findListingPda
} from "../utils/pnft";
import { PnftTransfer } from "../target/types/pnft_transfer";
import { PNftTransferClient } from "../utils/PNftTransferClient";
import { AnchorProvider } from "@project-serum/anchor";
import { NodeWallet } from "@metaplex/js";

let tx: Transaction;
let txid: string;
let tokenBalance;

describe("pnft_transfer tests", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  const connection = provider.connection;
  anchor.setProvider(provider);

  const PROG =  anchor.workspace.PnftTransfer as anchor.Program<PnftTransfer>;
  console.log(`>>>>> provider: ${JSON.stringify(provider.publicKey)}`);


  const pNftTransferClient = new PNftTransferClient(provider.connection, provider.wallet as anchor.Wallet)


  it.skip('transfers pnft to another account (no ruleset)', async () => {


    const nftOwner = await createFundedWallet(provider);
    const nftReceiver = await createFundedWallet(provider);

    // const nftOwner = Keypair.generate();
    // const nftReceiver = Keypair.generate();


    const creators = Array(5)
      .fill(null)
      .map((_) => ({ address: Keypair.generate().publicKey, share: 20 }));

    const { mint, ata } = await createAndFundATA({
      provider: provider,
      owner: nftOwner,
      creators,
      royaltyBps: 1000,
      programmable: true,
    });

    const destAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      nftReceiver,
      mint,
      nftReceiver.publicKey
    );
    const initialReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(initialReceiverBalance.value.uiAmount).to.equal(0)


    console.log('hello3');


    const builder = await pNftTransferClient.buildTransferPNFT({
      sourceAta: ata,
      nftMint: mint,
      destAta: destAta.address,
      owner: nftOwner.publicKey,
      receiver: nftReceiver.publicKey
    })
    await buildAndSendTx({
      provider,
      ixs: [await builder.instruction()],
      extraSigners: [nftOwner],
    });

    console.log('what..');

    const newReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(newReceiverBalance.value.uiAmount).to.equal(1)

  });


  it('deposits and withdraws a pnft', async () => {

    const creator = await createFundedWallet(provider);
    const buyer = await createFundedWallet(provider);

    const creators = Array(5)
      .fill(null)
      .map((_) => ({ address: Keypair.generate().publicKey, share: 20 }));

    const { mint, ata } = await createAndFundATA({
      provider: provider,
      owner: creator,
      creators,
      royaltyBps: 1000,
      programmable: true,
    });


    let [listingPda] = findListingPda(mint, PROG.programId);
    // listing's ata
    let listingItemToken = getAssociatedTokenAddressSync(mint, listingPda, true);

    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);
    let builder = await pNftTransferClient.buildListPNFT(price, {
      nftMint: mint,
      listing: listingPda,
      listingItemToken,
      authority: creator.publicKey,
    });
    await buildAndSendTx({
      provider,
      ixs: [await builder.instruction()],
      extraSigners: [creator],
    });

    console.log('what..');
    console.log(`buyer: ${buyer.publicKey.toBase58()}`);
    console.log(`listing: ${listingPda.toBase58()}`);
    console.log(`creator: ${creator.publicKey.toBase58()}`);

    tokenBalance = await connection.getTokenAccountBalance(listingItemToken);
    expect(tokenBalance.value.uiAmount).to.equal(1);

    // now making a withdraw
    const buyerProvider = new AnchorProvider(connection, new NodeWallet(buyer), provider.opts);

    const buyerItemToken = getAssociatedTokenAddressSync(mint, buyer.publicKey, true);

    tx = new Transaction();
    tx.add(
      createAssociatedTokenAccountInstruction(buyer.publicKey, buyerItemToken, buyer.publicKey, mint)
    );
    txid = await buyerProvider.sendAndConfirm(tx);
    console.log(`created buyer's ata ${buyerItemToken}: ${txid}`);

    builder = await pNftTransferClient.buildBuyPNFT( {
      nftMint: mint,
      listing: listingPda,
      listingItemToken,
      buyer: buyer.publicKey,
    });
    txid = await buildAndSendTx({
      provider: buyerProvider,
      ixs: [await builder.instruction()],
      extraSigners: [buyer],
    });

    console.log(` ------->>>>> withdrew txid: ${txid}`);




  });


  it.skip('transfers pnft to another account (1 ruleset)', async () => {
    const nftOwner = await createFundedWallet(provider);

    const name = 'PlayRule123';

    const ruleSetAddr = await createTokenAuthorizationRules(
      provider,
      nftOwner,
      name
    );


    const nftReceiver = await createFundedWallet(provider);

    const creators = Array(5)
      .fill(null)
      .map((_) => ({ address: Keypair.generate().publicKey, share: 20 }));

    const { mint, ata } = await createAndFundATA({
      provider: provider,
      owner: nftOwner,
      creators,
      royaltyBps: 1000,
      programmable: true,
      ruleSetAddr
    });

    const destAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      nftReceiver,
      mint,
      nftReceiver.publicKey
    );
    const initialReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(initialReceiverBalance.value.uiAmount).to.equal(0)

    const builder = await pNftTransferClient.buildTransferPNFT({
      sourceAta: ata,
      nftMint: mint,
      destAta: destAta.address,
      owner: nftOwner.publicKey,
      receiver: nftReceiver.publicKey
    })
    await buildAndSendTx({
      provider,
      ixs: [await builder.instruction()],
      extraSigners: [nftOwner],
    });

    const newReceiverBalance = await provider.connection.getTokenAccountBalance(destAta.address)
    expect(newReceiverBalance.value.uiAmount).to.equal(1)

  });
});
