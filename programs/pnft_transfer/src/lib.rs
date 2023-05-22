use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_token_auth_rules::payload::{Payload, PayloadType, ProofInfo, SeedsVec};
use mpl_token_metadata::{
    self,
    instruction::{builders::TransferBuilder, InstructionBuilder, TransferArgs},
    processor::AuthorizationData,
    state::{Metadata, ProgrammableConfig::V1, TokenMetadataAccount, TokenStandard},
};
pub mod errors;
pub mod utils;

use errors::ErrorCode;
use utils::*;

declare_id!("4VL7z3sVLTEUt6NCbey5FxWSvwQrN7Yf9LXXjZz538wA");

#[program]
pub mod pnft_transfer {
    use mpl_token_metadata::state::PayloadKey;
    use super::*;

    pub fn transfer_pnft<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferPNFT<'info>>,
        authorization_data: Option<AuthorizationDataLocal>,
        rules_acc_present: bool,
    ) -> Result<()> {
        let rem_acc = &mut ctx.remaining_accounts.iter();
        let auth_rules = if rules_acc_present {
            Some(next_account_info(rem_acc)?)
        } else {
            None
        };
        send_pnft(
            &ctx.accounts.owner.to_account_info(),
            &ctx.accounts.owner.to_account_info(),
            &ctx.accounts.src,
            &ctx.accounts.dest,
            &ctx.accounts.receiver.to_account_info(),
            &ctx.accounts.nft_mint,
            &ctx.accounts.nft_metadata,
            &ctx.accounts.edition,
            &ctx.accounts.system_program,
            &ctx.accounts.token_program,
            &ctx.accounts.associated_token_program,
            &ctx.accounts.pnft_shared.instructions,
            &ctx.accounts.owner_token_record,
            &ctx.accounts.dest_token_record,
            &ctx.accounts.pnft_shared.authorization_rules_program,
            auth_rules,
            authorization_data,
            // None,
        )?;
        Ok(())
    }


    pub fn list_pnft<'info>(
        ctx: Context<'_, '_, '_, 'info, ListPNFT<'info>>,
        price: u64,
        authorization_data: Option<AuthorizationDataLocal>,
        rules_acc_present: bool,
    ) -> Result<()> {

        // make sure the item exists in the from account
        require!(ctx.accounts.authority_item_token.amount == 1, ErrorCode::InvalidItem);

        // first, transfer the item to the listing ata
        let rem_acc = &mut ctx.remaining_accounts.iter();
        let auth_rules = if rules_acc_present {
            Some(next_account_info(rem_acc)?)
        } else {
            None
        };
        send_pnft(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.authority_item_token,
            &ctx.accounts.listing_item_token,
            &ctx.accounts.listing.to_account_info(),
            &ctx.accounts.item,
            &ctx.accounts.item_metadata,
            &ctx.accounts.edition,
            &ctx.accounts.system_program,
            &ctx.accounts.token_program,
            &ctx.accounts.associated_token_program,
            &ctx.accounts.instructions,
            &ctx.accounts.authority_token_record,
            &ctx.accounts.listing_token_record,
            &ctx.accounts.authorization_rules_program,
            auth_rules,
            authorization_data,
        )?;

        // now create the listing
        let listing = &mut ctx.accounts.listing;
        listing.item = ctx.accounts.item.key();
        listing.item_token = ctx.accounts.listing_item_token.key();
        listing.bump = *ctx.bumps.get("listing").unwrap();

        Ok(())
    }


    pub fn buy_pnft<'info>(
        ctx: Context<'_, '_, '_, 'info, BuyPNFT<'info>>,
    ) -> Result<()> {
        msg!("Withdraw");


        /*
        let account_iter = &mut accounts.iter();
        let authority_info = next_account_info(account_iter)?;
        let rooster_pda_info = next_account_info(account_iter)?;
        let token_info = next_account_info(account_iter)?;
        let destination_owner_info = next_account_info(account_iter)?;
        let destination_info = next_account_info(account_iter)?;
        let mint_info = next_account_info(account_iter)?;
        let metadata_info = next_account_info(account_iter)?;
        let edition_info = next_account_info(account_iter)?;
        let owner_token_record_info = next_account_info(account_iter)?;
        let destination_token_record_info = next_account_info(account_iter)?;
        let token_metadata_program_info = next_account_info(account_iter)?;
        let system_program_info = next_account_info(account_iter)?;
        let sysvar_instructions_info = next_account_info(account_iter)?;
        let spl_token_program_info = next_account_info(account_iter)?;
        let spl_ata_program_info = next_account_info(account_iter)?;
        let mpl_token_auth_rules_program_info = next_account_info(account_iter)?;
        let rule_set_info = next_account_info(account_iter)?;

         */

        let listing = &ctx.accounts.listing;


        // let auth_data = Some(AuthorizationData::try_from(authorization_data).unwrap());

        let seeds = SeedsVec {
            seeds: vec![ctx.accounts.item.key().as_ref().to_vec(), String::from("listings").as_bytes().to_vec()]
        };

        let mut payload = Payload::new();
        payload.insert(PayloadKey::SourceSeeds.to_string(), PayloadType::Seeds(seeds));


        let auth_data = Some(AuthorizationData {
            payload
        });

        msg!("auth data: {:?}", auth_data);

        let transfer_args = TransferArgs::V1 {
            amount: 1,
            authorization_data: auth_data
            // authorization_data: None,
        };

        msg!("setting up builder");
        let mut builder = TransferBuilder::new();
        builder
            .authority(ctx.accounts.listing.key())
            .token_owner(ctx.accounts.listing.key())
            .token(ctx.accounts.listing_item_token.key())
            .destination_owner(ctx.accounts.buyer.key())
            .destination(ctx.accounts.buyer_item_token.key())
            .mint(ctx.accounts.item.key())
            .metadata(ctx.accounts.item_metadata.key())
            .edition(ctx.accounts.edition.key())
            .owner_token_record(ctx.accounts.listing_token_record.key())
            .destination_token_record(ctx.accounts.buyer_token_record.key())
            // .authorization_rules(ctx.accounts.ruleset.key())
            .authorization_rules_program(ctx.accounts.authorization_rules_program.key())
            .payer(ctx.accounts.buyer.key());

        msg!("building transfer instruction");
        let build_result = builder.build(transfer_args);

        let instruction = match build_result {
            Ok(transfer) => {
                msg!("transfer instruction built");
                transfer.instruction()
            }
            Err(err) => {
                msg!("Error building transfer instruction: {:?}", err);
                return Err(ErrorCode::TransferBuilderFailed.into());
            }
        };

        let account_infos = [
            ctx.accounts.listing.to_account_info(),
            ctx.accounts.listing_item_token.to_account_info(),
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.buyer_item_token.to_account_info(),
            ctx.accounts.item.to_account_info(),
            ctx.accounts.item_metadata.to_account_info(),
            ctx.accounts.edition.to_account_info(),
            ctx.accounts.listing_token_record.to_account_info(),
            ctx.accounts.buyer_token_record.to_account_info(),
            ctx.accounts.ruleset.to_account_info(),
            ctx.accounts.listing.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.instructions.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.associated_token_program.to_account_info(),
            ctx.accounts.authorization_rules_program.to_account_info(),
        ];

        msg!("invoking transfer instruction");

        let signer_seeds = &[listing.item.as_ref(), b"listings".as_ref(), &[listing.bump]];

        invoke_signed(&instruction, &account_infos, &[signer_seeds]).unwrap();

        Ok(())
    }

}

#[derive(Accounts)]
pub struct BuyPNFT<'info> {

    pub item: Box<Account<'info, Mint>>,

    #[account(
        mut,
        token::mint = item,
        token::authority = buyer
    )]
    pub buyer_item_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        has_one = item,
        constraint = listing.item == item.key() && listing.item_token == listing_item_token.key(),
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        mut,
        associated_token::mint = item,
        associated_token::authority = listing
    )]
    pub listing_item_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    // programs
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // pnft shit

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: assert_decode_metadata + seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub item_metadata: UncheckedAccount<'info>,

    //note that MASTER EDITION and EDITION share the same seeds, and so it's valid to check them here
    /// CHECK: seeds below
    #[account(
    mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::EDITION.as_bytes(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub edition: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
            buyer_item_token.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub buyer_token_record: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
            listing_item_token.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub listing_token_record: UncheckedAccount<'info>,

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: address below
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: UncheckedAccount<'info>,

    //sysvar ixs don't deserialize in anchor
    /// CHECK: address below
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    /// CHECK: address below
    #[account(address = mpl_token_auth_rules::id())]
    pub authorization_rules_program: UncheckedAccount<'info>,

    /// CHECK:
    #[account()]
    pub ruleset: UncheckedAccount<'info>,
}


#[derive(Accounts)]
pub struct ListPNFT<'info> {

    pub item: Box<Account<'info, Mint>>,

    #[account(
        mut,
        token::mint = item,
        token::authority = authority
    )]
    pub authority_item_token: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = authority,
        seeds = [item.key().as_ref(), "listings".as_bytes().as_ref()],
        bump,
        space = 8 + 128,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = item,
        associated_token::authority = listing
    )]
    pub listing_item_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    // programs
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // pnft shit

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: assert_decode_metadata + seeds below
    #[account(
    mut,
    seeds=[
    mpl_token_metadata::state::PREFIX.as_bytes(),
    mpl_token_metadata::id().as_ref(),
    item.key().as_ref(),
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub item_metadata: UncheckedAccount<'info>,

    //note that MASTER EDITION and EDITION share the same seeds, and so it's valid to check them here
    /// CHECK: seeds below
    #[account(
    seeds=[
    mpl_token_metadata::state::PREFIX.as_bytes(),
    mpl_token_metadata::id().as_ref(),
    item.key().as_ref(),
    mpl_token_metadata::state::EDITION.as_bytes(),
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub edition: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
    mut,
    seeds=[
    mpl_token_metadata::state::PREFIX.as_bytes(),
    mpl_token_metadata::id().as_ref(),
    item.key().as_ref(),
    mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
    authority_item_token.key().as_ref()
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub authority_token_record: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
    mut,
    seeds=[
    mpl_token_metadata::state::PREFIX.as_bytes(),
    mpl_token_metadata::id().as_ref(),
    item.key().as_ref(),
    mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
    listing_item_token.key().as_ref()
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub listing_token_record: UncheckedAccount<'info>,

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: address below
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: UncheckedAccount<'info>,

    //sysvar ixs don't deserialize in anchor
    /// CHECK: address below
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    /// CHECK: address below
    #[account(address = mpl_token_auth_rules::id())]
    pub authorization_rules_program: UncheckedAccount<'info>,
}



#[derive(Accounts)]
pub struct TransferPNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK:
    pub receiver: AccountInfo<'info>,
    #[account(mut)]
    pub src: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub dest: Box<Account<'info, TokenAccount>>,
    pub nft_mint: Box<Account<'info, Mint>>,
    // misc
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // pfnt
    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: assert_decode_metadata + seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            nft_mint.key().as_ref(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub nft_metadata: UncheckedAccount<'info>,
    //note that MASTER EDITION and EDITION share the same seeds, and so it's valid to check them here
    /// CHECK: seeds below
    #[account(
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            nft_mint.key().as_ref(),
            mpl_token_metadata::state::EDITION.as_bytes(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub edition: UncheckedAccount<'info>,
    /// CHECK: seeds below
    #[account(mut,
            seeds=[
                mpl_token_metadata::state::PREFIX.as_bytes(),
                mpl_token_metadata::id().as_ref(),
                nft_mint.key().as_ref(),
                mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
                src.key().as_ref()
            ],
            seeds::program = mpl_token_metadata::id(),
            bump
        )]
    pub owner_token_record: UncheckedAccount<'info>,
    /// CHECK: seeds below
    #[account(mut,
            seeds=[
                mpl_token_metadata::state::PREFIX.as_bytes(),
                mpl_token_metadata::id().as_ref(),
                nft_mint.key().as_ref(),
                mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
                dest.key().as_ref()
            ],
            seeds::program = mpl_token_metadata::id(),
            bump
        )]
    pub dest_token_record: UncheckedAccount<'info>,
    pub pnft_shared: ProgNftShared<'info>,
    //
    // remaining accounts could be passed, in this order:
    // - rules account
    // - mint_whitelist_proof
    // - creator_whitelist_proof
}

#[derive(Accounts)]
pub struct ProgNftShared<'info> {
    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: address below
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: UncheckedAccount<'info>,

    //sysvar ixs don't deserialize in anchor
    /// CHECK: address below
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    /// CHECK: address below
    #[account(address = mpl_token_auth_rules::id())]
    pub authorization_rules_program: UncheckedAccount<'info>,
}
// --------------------------------------- replicating mplex type for anchor IDL export
//have to do this because anchor won't include foreign structs in the IDL

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct AuthorizationDataLocal {
    pub payload: Vec<TaggedPayload>,
}
impl From<AuthorizationDataLocal> for AuthorizationData {
    fn from(val: AuthorizationDataLocal) -> Self {
        let mut p = Payload::new();
        val.payload.into_iter().for_each(|tp| {
            p.insert(tp.name, PayloadType::try_from(tp.payload).unwrap());
        });
        AuthorizationData { payload: p }
    }
}

//Unfortunately anchor doesn't like HashMaps, nor Tuples, so you can't pass in:
// HashMap<String, PayloadType>, nor
// Vec<(String, PayloadTypeLocal)>
// so have to create this stupid temp struct for IDL to serialize correctly
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct TaggedPayload {
    name: String,
    payload: PayloadTypeLocal,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub enum PayloadTypeLocal {
    /// A plain `Pubkey`.
    Pubkey(Pubkey),
    /// PDA derivation seeds.
    Seeds(SeedsVecLocal),
    /// A merkle proof.
    MerkleProof(ProofInfoLocal),
    /// A plain `u64` used for `Amount`.
    Number(u64),
}
impl From<PayloadTypeLocal> for PayloadType {
    fn from(val: PayloadTypeLocal) -> Self {
        match val {
            PayloadTypeLocal::Pubkey(pubkey) => PayloadType::Pubkey(pubkey),
            PayloadTypeLocal::Seeds(seeds) => {
                msg!(">>> seeds: {:?}", seeds);
                PayloadType::Seeds(SeedsVec::try_from(seeds).unwrap())
            }
            PayloadTypeLocal::MerkleProof(proof) => {
                PayloadType::MerkleProof(ProofInfo::try_from(proof).unwrap())
            }
            PayloadTypeLocal::Number(number) => PayloadType::Number(number),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct SeedsVecLocal {
    /// The vector of derivation seeds.
    pub seeds: Vec<Vec<u8>>,
}
impl From<SeedsVecLocal> for SeedsVec {
    fn from(val: SeedsVecLocal) -> Self {
        SeedsVec { seeds: val.seeds }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct ProofInfoLocal {
    /// The merkle proof.
    pub proof: Vec<[u8; 32]>,
}
impl From<ProofInfoLocal> for ProofInfo {
    fn from(val: ProofInfoLocal) -> Self {
        ProofInfo { proof: val.proof }
    }
}

#[account]
pub struct Listing {

    // these are for doing gPA lookups
    pub bump: u8,
    // todo: add collection for lookups too
    // pub collection: Pubkey,

    pub item: Pubkey,
    pub item_token: Pubkey,
}

