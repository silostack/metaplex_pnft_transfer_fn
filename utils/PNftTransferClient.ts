import { Metaplex } from "@metaplex-foundation/js";
import {
    AuthorizationData,
    Metadata,
    PROGRAM_ID as TMETA_PROG_ID,
} from "@metaplex-foundation/mpl-token-metadata";
import { PROGRAM_ID as AUTH_PROG_ID } from '@metaplex-foundation/mpl-token-auth-rules';
import * as anchor from "@project-serum/anchor";
import { Idl } from "@project-serum/anchor";
import { Connection, PublicKey, SystemProgram, SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import { PnftTransfer } from "../target/types/pnft_transfer";
import { fetchNft, findTokenRecordPDA } from "./pnft";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";

export class PNftTransferClient  {

    wallet: anchor.Wallet;
    provider!: anchor.Provider;
    program!: anchor.Program<PnftTransfer>;
    connection: Connection;
    constructor(
        connection: Connection,
        wallet: anchor.Wallet,
        idl?: Idl,
        programId?: PublicKey
    ) {
        this.wallet = wallet;
        this.connection = connection;
        this.setProvider();
        this.setProgram(idl, programId);
    }

    setProvider() {
        this.provider = new anchor.AnchorProvider(
            this.connection,
            this.wallet,
            anchor.AnchorProvider.defaultOptions()
        );
        anchor.setProvider(this.provider);
    }

    setProgram(idl?: Idl, programId?: PublicKey) {
        //instantiating program depends on the environment
        if (idl && programId) {
            //means running in prod
            this.program = new anchor.Program<PnftTransfer>(
                idl as any,
                programId,
                this.provider
            );
        } else {
            //means running inside test suite
            this.program = anchor.workspace.PnftTransfer as anchor.Program<PnftTransfer>;
        }
    }

    async prepPnftAccounts({
        nftMetadata,
        nftMint,
        sourceAta,
        destAta,
        authData = null,
    }: {
        nftMetadata?: PublicKey;
        nftMint: PublicKey;
        sourceAta: PublicKey;
        destAta: PublicKey;
        authData?: AuthorizationData | null;
    }) {
        let meta;
        let creators: PublicKey[] = [];
        if (nftMetadata) {
            meta = nftMetadata;
        } else {
            const nft = await fetchNft(this.provider.connection, nftMint);
            meta = nft.metadataAddress;
            creators = nft.creators.map((c) => c.address);
        }

        const inflatedMeta = await Metadata.fromAccountAddress(
            this.provider.connection,
            meta
        );
        const ruleSet = inflatedMeta.programmableConfig?.ruleSet;

        const [ownerTokenRecordPda, ownerTokenRecordBump] =
            await findTokenRecordPDA(nftMint, sourceAta);
        const [destTokenRecordPda, destTokenRecordBump] = await findTokenRecordPDA(
            nftMint,
            destAta
        );

        //retrieve edition PDA
        const mplex = new Metaplex(this.provider.connection);
        const nftEditionPda = mplex.nfts().pdas().edition({ mint: nftMint });

        //have to re-serialize due to anchor limitations
        const authDataSerialized = authData
            ? {
                payload: Object.entries(authData.payload.map).map(([k, v]) => {
                    return { name: k, payload: v };
                }),
            }
            : null;

        return {
            meta,
            creators,
            ownerTokenRecordBump,
            ownerTokenRecordPda,
            destTokenRecordBump,
            destTokenRecordPda,
            ruleSet,
            nftEditionPda,
            authDataSerialized,
        };
    }


    async buildTransferPNFT({
        nftMint,
        sourceAta,
        destAta,
        owner,
        receiver
    }: {
        nftMint: PublicKey;
        sourceAta: PublicKey;
        destAta: PublicKey;
        owner: PublicKey;
        receiver: PublicKey;
    }) {
        //pnft
        const {
            meta,
            ownerTokenRecordBump,
            ownerTokenRecordPda,
            destTokenRecordBump,
            destTokenRecordPda,
            ruleSet,
            nftEditionPda,
            authDataSerialized,
        } = await this.prepPnftAccounts({
            nftMint,
            destAta,
            authData: null, //currently useless
            sourceAta,
        });
        const remainingAccounts = [];
        if (!!ruleSet) {
            remainingAccounts.push({
                pubkey: ruleSet,
                isSigner: false,
                isWritable: false,
            });
        }

        const builder = this.program.methods
            .transferPnft(authDataSerialized, !!ruleSet)
            .accounts({
                owner,
                src: sourceAta,
                dest: destAta,
                ownerTokenRecord: ownerTokenRecordPda,
                destTokenRecord: destTokenRecordPda,
                nftMint,
                edition: nftEditionPda,
                nftMetadata: meta,
                receiver,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                pnftShared: {
                    authorizationRulesProgram: AUTH_PROG_ID,
                    tokenMetadataProgram: TMETA_PROG_ID,
                    instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
                }
            })
            .remainingAccounts(remainingAccounts)

        return builder
    }


    async buildListPNFT(priceBN, {
        nftMint,
        listing,
        listingItemToken,
        authority}: {
        keychain: PublicKey;
        nftMint: PublicKey;
        listingItemToken: PublicKey;
        authority: PublicKey;
        listing: PublicKey;
    }) {

        const authorityItemToken = getAssociatedTokenAddressSync(nftMint, authority);

        //pnft
        const {
            meta,
            ownerTokenRecordBump,
            ownerTokenRecordPda,
            destTokenRecordBump,
            destTokenRecordPda,
            ruleSet,
            nftEditionPda,
            authDataSerialized,
        } = await this.prepPnftAccounts({
            nftMint,
            destAta: listingItemToken,
            authData: null, //currently useless
            sourceAta: authorityItemToken,
        });
        const remainingAccounts = [];
        if (!!ruleSet) {
            remainingAccounts.push({
                pubkey: ruleSet,
                isSigner: false,
                isWritable: false,
            });
        }

        console.log(`>> itemMetadata: ${meta.toBase58()}`);
        console.log(`>> edition: ${nftEditionPda.toBase58()}`);
        console.log(`>> authorityTokenRecord: ${ownerTokenRecordPda.toBase58()}`);
        console.log(`>> authorityItemToken: ${authorityItemToken.toBase58()}`);
        console.log(`>> listingItemToken: ${listingItemToken.toBase58()}`);
        console.log(`>> nftMint: ${nftMint.toBase58()}`);


        const builder = this.program.methods
          .listPnft(priceBN, authDataSerialized, !!ruleSet)
          .accounts({
              item: nftMint,
              authorityItemToken,
              listing,
              listingItemToken,
              authority,
              tokenProgram: TOKEN_PROGRAM_ID,
              systemProgram: SystemProgram.programId,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
              associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
              itemMetadata: meta,
              edition: nftEditionPda,
              authorityTokenRecord: ownerTokenRecordPda,
              listingTokenRecord: destTokenRecordPda,
              tokenMetadataProgram: TMETA_PROG_ID,
              instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
              authorizationRulesProgram: AUTH_PROG_ID,
          })
          .remainingAccounts(remainingAccounts);

        return builder
    }

    async buildBuyPNFT({nftMint,
                        listing,
                        listingItemToken,
                        buyer}: {
        nftMint: PublicKey;
        listingItemToken: PublicKey;
        buyer: PublicKey;
        listing: PublicKey;
    }) {

        const buyerItemToken = getAssociatedTokenAddressSync(nftMint, buyer);

        //pnft
        const {
            meta,
            ownerTokenRecordBump,
            ownerTokenRecordPda,
            destTokenRecordBump,
            destTokenRecordPda,
            ruleSet,
            nftEditionPda,
            authDataSerialized,
        } = await this.prepPnftAccounts({
            nftMint,
            destAta: buyerItemToken,
            authData: null, //currently useless
            sourceAta: listingItemToken,
        });
        const remainingAccounts = [];
        if (!!ruleSet) {
            remainingAccounts.push({
                pubkey: ruleSet,
                isSigner: false,
                isWritable: false,
            });
        }

        console.log(`>> itemMetadata: ${meta.toBase58()}`);
        console.log(`>> edition: ${nftEditionPda.toBase58()}`);
        console.log(`>> buyerTokenRecord: ${ownerTokenRecordPda.toBase58()}`);
        console.log(`>> buyerItemToken: ${buyerItemToken.toBase58()}`);
        console.log(`>> nftMint: ${nftMint.toBase58()}`);
        console.log(`>> authdata serialized: ${authDataSerialized}`);
        if (!!ruleSet) {
            console.log(`>> ruleset: ${ruleSet.toBase58()}`);
        } else {
            console.log(`>> no ruleset`);
        }

      const map = new Map();
      map.set('SourceSeeds', {
        __kind: 'Seeds',
        fields: [
          {
            seeds: [nftMint.toBuffer(), Buffer.from("listings")],
          },
        ],

      });

      const authorizationData = {
        payload: [{
          // __kind: 'Seeds',
          name: "SourceSeeds",
          payload: {
            name: "Seeds",
            seeds: [nftMint.toBuffer(), Buffer.from("listings")]
          },
        }]
      };


      console.log(`>> authorizationData: ${JSON.stringify(authorizationData)}`);


      const builder = this.program.methods
          // .buyPnft(authDataSerialized, !!ruleSet)
          .buyPnft()
          .accounts({
              listing,
              item: nftMint,
              listingItemToken,
              buyerItemToken,
              buyer,
              associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
              tokenProgram: TOKEN_PROGRAM_ID,
              systemProgram: SystemProgram.programId,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
              itemMetadata: meta,
              edition: nftEditionPda,
              listingTokenRecord: ownerTokenRecordPda,
              buyerTokenRecord: destTokenRecordPda,
              tokenMetadataProgram: TMETA_PROG_ID,
              instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
              authorizationRulesProgram: AUTH_PROG_ID,
              ruleset: new PublicKey('eBJLFYPxJmMGKuFwpDWkzxZeUrad92kZRC5BJLpzyT9'),
          });

        if (remainingAccounts.length > 0) {
            console.log("!! adding remaining accounts !!");
            builder.remainingAccounts(remainingAccounts);
        }

        return builder;
    }


}
