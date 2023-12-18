import * as anchor from "@coral-xyz/anchor";
import { IDL, NftEscrow } from "../target/types/nft_escrow";

import { PublicKey, 
  Commitment,
  SystemProgram, 
  LAMPORTS_PER_SOL } from "@solana/web3.js";

import {
  createFungible,
  createNft, 
  createProgrammableNft, 
  mplTokenMetadata
} from "@metaplex-foundation/mpl-token-metadata";

import {
  getOrCreateAssociatedTokenAccount,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  mintTo
} from "@solana/spl-token";

import {Connection} from "@solana/web3.js";

import { 
  MPL_TOKEN_METADATA_PROGRAM_ID
} from '@metaplex-foundation/mpl-token-metadata';

import { createUmi } from "@metaplex-foundation/umi-bundle-defaults"

import { base58 } from "@metaplex-foundation/umi/serializers";

import { 
  createSignerFromKeypair, 
  generateSigner, 
  percentAmount, 
  signerIdentity 
} from "@metaplex-foundation/umi";


describe("anchor-nft-escrow", () => {
  const maker = anchor.web3.Keypair.generate();
  const taker = anchor.web3.Keypair.generate();

  const commitment: Commitment = "confirmed"; // processed, confirmed, finalized
  const connection = new Connection("http://localhost:8899", {
      commitment,
      wsEndpoint: "ws://localhost:8900/",
  });
  const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(maker), { commitment });
  const programId = new PublicKey("2VYDrwoKRKNgvmQo3DHfcLfQFAuijjNEAY9ZoXfv8GfZ");
  const program = new anchor.Program<NftEscrow>(IDL, programId, provider);

  // Helpers
  function wait(ms: number) {
    return new Promise( resolve => setTimeout(resolve, ms) );
  }

  const confirm = async (signature: string): Promise<string> => {
    const block = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...block
    })
    return signature
  }

  const log = async(signature: string): Promise<string> => {
    console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`);
    return signature;
  }

  //Variables
  let makerAtaA: anchor.web3.PublicKey;
  let makerAtaB: anchor.web3.PublicKey;

  let takerAtaA: anchor.web3.PublicKey;
  let takerAtaB: anchor.web3.PublicKey;

  let mintA: anchor.web3.PublicKey;
  let metadataA: anchor.web3.PublicKey;
  let masterEditionA: anchor.web3.PublicKey;
  let makerTokenRecordA: anchor.web3.PublicKey;
  let vaultTokenRecordA: anchor.web3.PublicKey;
  let takerTokenRecordA: anchor.web3.PublicKey;


  let mintB: anchor.web3.PublicKey;
  let metadataB: anchor.web3.PublicKey;
  let masterEditionB: anchor.web3.PublicKey;
  let makerTokenRecordB: anchor.web3.PublicKey;
  let takerTokenRecordB: anchor.web3.PublicKey;

  let escrow: anchor.web3.PublicKey;
  let vault: anchor.web3.PublicKey;


  it("Airdrop", async () => {
    await connection.requestAirdrop(maker.publicKey, LAMPORTS_PER_SOL * 100)
    .then(confirm)
    .then(log)

    await connection.requestAirdrop(taker.publicKey, LAMPORTS_PER_SOL * 100)
    .then(confirm)
    .then(log)
  })

  describe("FT escrow", () => {

    it("Creates a FtA", async () => {
      const umi = createUmi(connection.rpcEndpoint);
      let umiKeypair = umi.eddsa.createKeypairFromSecretKey(maker.secretKey);
      const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
      umi.use(signerIdentity(signerKeypair));
      umi.use(mplTokenMetadata())
      const mint = generateSigner(umi);
      mintA = new PublicKey(mint.publicKey)
  
      // Create NFT
      let tx = createFungible(
        umi, 
        {
          mint: mint,
          authority: signerKeypair,
          updateAuthority: umiKeypair.publicKey,
          name: "NFT Example",
          symbol: "EXM",
          uri: "",
          sellerFeeBasisPoints: percentAmount(0),
          creators: [
              {address: umiKeypair.publicKey, verified: true, share: 100 }
          ],
          collection: null,
          uses: null,
          isMutable: true,
          collectionDetails: null,
        }
      );
  
      const result = await tx.sendAndConfirm(umi, {
        send: {
          skipPreflight: true
        },
        confirm: {
          commitment
        }
      });
  
      const signature = base58.deserialize(result.signature);
      console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)
  
      let ata = await getOrCreateAssociatedTokenAccount(
        connection,
        maker,
        mintA,
        maker.publicKey
      );
      makerAtaA = ata.address;
  
      let mintTx = await mintTo(
        connection,
        maker,
        mintA,
        makerAtaA,
        maker.publicKey,
        100,
      )
  
      takerAtaA = await getAssociatedTokenAddressSync(
        mintA,
        taker.publicKey,
      );
  
      const metadata_seeds = [
        Buffer.from('metadata'),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
      ];
  
      metadataA = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
    });
  
    it("Creates a FtB", async () => {
      const umi = createUmi(connection.rpcEndpoint);
      let umiKeypair = umi.eddsa.createKeypairFromSecretKey(taker.secretKey);
      const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
      umi.use(signerIdentity(signerKeypair));
      umi.use(mplTokenMetadata())
      const mint = generateSigner(umi);
      mintB = new PublicKey(mint.publicKey)
  
      // Create NFT
      let minttx = createFungible(
        umi, 
        {
          mint: mint,
          authority: signerKeypair,
          updateAuthority: umiKeypair.publicKey,
          name: "NFT Example",
          symbol: "EXM",
          uri: "",
          sellerFeeBasisPoints: percentAmount(0),
          creators: [
              {address: umiKeypair.publicKey, verified: true, share: 100 }
          ],
          collection: null,
          uses: null,
          isMutable: true,
          collectionDetails: null,
        }
      );
  
      const result = await minttx.sendAndConfirm(umi, {
        send: {
          skipPreflight: true
        },
        confirm: {
          commitment
        }
      });
  
      const signature = base58.deserialize(result.signature);
      console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)
  
      makerAtaB = await getAssociatedTokenAddressSync(
        mintB,
        maker.publicKey,
      );

      let ata = await getOrCreateAssociatedTokenAccount(
        connection,
        taker,
        mintB,
        taker.publicKey
      );
      takerAtaB = ata.address;
  
      let mintTx = await mintTo(
        connection,
        taker,
        mintB,
        takerAtaB,
        taker.publicKey,
        100,
      )
  
      const metadata_seeds = [
        Buffer.from('metadata'),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
      ];
  
      metadataB = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
    });
    
    it("Make", async () => {
      const escrow_seeds = [
        Buffer.from('escrow'),
        new PublicKey(maker.publicKey).toBuffer(),
        new PublicKey(mintA).toBuffer(),
        new PublicKey(mintB).toBuffer(),
      ];

      escrow = await PublicKey.findProgramAddressSync(escrow_seeds, programId)[0];

      vault = await getAssociatedTokenAddressSync(
        mintA,
        escrow,
        true
      );

      const signature = await program.methods
      .make(new anchor.BN(1))
      .accounts({
        maker: maker.publicKey,
        makerAta: makerAtaA,
        mintA,
        mintB,
        metadataA,
        masterEditionA: null,
        makerTokenRecordA: null,
        vaultTokenRecordA: anchor.web3.Keypair.generate().publicKey,
        vault,
        escrow,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker]).rpc({skipPreflight: true}).then(confirm).then(log);
    });
    
    xit("Close", async () => {

      const signature = await program.methods
      .close()
      .accounts({
        maker: maker.publicKey,
        makerAta: makerAtaA,
        mintA,
        mintB,
        metadataA,
        masterEditionA: null,
        makerTokenRecordA: anchor.web3.Keypair.generate().publicKey,
        vaultTokenRecordA: null,
        vault,
        escrow,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker]).rpc({skipPreflight: true}).then(confirm).then(log);
    });

    it("Take", async () => {

      const signature = await program.methods
      .take(new anchor.BN(1))
      .accounts({
        taker: taker.publicKey,
        maker: maker.publicKey,
        mintA,
        mintB,
        metadataA,
        masterEditionA: null,
        vaultTokenRecordA: null,
        takerTokenRecordA: anchor.web3.Keypair.generate().publicKey,
        metadataB,
        masterEditionB: null,
        takerTokenRecordB: null,
        makerTokenRecordB: anchor.web3.Keypair.generate().publicKey,
        vault,
        takerAtaA,
        takerAtaB,
        makerAtaB,
        escrow,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker]).rpc({skipPreflight: true}).then(confirm).then(log);
    });
  });

  xdescribe("NFT escrow", () => {
    
    it("Creates a NftA", async () => {
      // Metaplex Setup
      const umi = createUmi(connection.rpcEndpoint);
      let umiKeypair = umi.eddsa.createKeypairFromSecretKey(maker.secretKey);
      const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
      umi.use(signerIdentity(signerKeypair));
      umi.use(mplTokenMetadata())
      const mint = generateSigner(umi);
      mintA = new PublicKey(mint.publicKey)
  
      // Create NFT
      let minttx = createNft(
        umi, 
        {
          mint: mint,
          authority: signerKeypair,
          updateAuthority: umiKeypair.publicKey,
          name: "NFT Example",
          symbol: "EXM",
          uri: "",
          sellerFeeBasisPoints: percentAmount(0),
          creators: [
              {address: umiKeypair.publicKey, verified: true, share: 100 }
          ],
          collection: null,
          uses: null,
          isMutable: true,
          collectionDetails: null,
        }
      );
  
      const result = await minttx.sendAndConfirm(umi, {
        send: {
          skipPreflight: true
        },
        confirm: {
          commitment
        }
      });
  
      const signature = base58.deserialize(result.signature);
      console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)
  
      // Create Collection Accounts
      let ata = await getOrCreateAssociatedTokenAccount(
        connection,
        maker,
        mintA,
        maker.publicKey
      );
  
      makerAtaA = ata.address;
  
      ata = await getOrCreateAssociatedTokenAccount(
        connection,
        maker,
        mintA,
        taker.publicKey,
      );
  
      takerAtaA = ata.address;
  
      const metadata_seeds = [
        Buffer.from('metadata'),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
      ];
  
      const master_edition_seeds = [
        ...metadata_seeds,
        Buffer.from("edition")
      ];
  
      metadataA = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
      masterEditionA = PublicKey.findProgramAddressSync(master_edition_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0]; 
    });
  
    it("Creates a NftB", async () => {
      // Metaplex Setup
      const umi = createUmi(connection.rpcEndpoint);
      let umiKeypair = umi.eddsa.createKeypairFromSecretKey(taker.secretKey);
      const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
      umi.use(signerIdentity(signerKeypair));
      umi.use(mplTokenMetadata())
      const mint = generateSigner(umi);
      mintB = new PublicKey(mint.publicKey)
  
      // Create NFT
      let minttx = createNft(
        umi, 
        {
          mint: mint,
          authority: signerKeypair,
          updateAuthority: umiKeypair.publicKey,
          name: "NFT Example",
          symbol: "EXM",
          uri: "",
          sellerFeeBasisPoints: percentAmount(0),
          creators: [
              {address: umiKeypair.publicKey, verified: true, share: 100 }
          ],
          collection: null,
          uses: null,
          isMutable: true,
          collectionDetails: null,
        }
      );
  
      const result = await minttx.sendAndConfirm(umi, {
        send: {
          skipPreflight: true
        },
        confirm: {
          commitment
        }
      });
  
      const signature = base58.deserialize(result.signature);
      console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)
  
      // Create Collection Accounts
      let ata = await getOrCreateAssociatedTokenAccount(
        connection,
        taker,
        mintB,
        maker.publicKey
      );
  
      makerAtaB = ata.address;
  
      ata = await getOrCreateAssociatedTokenAccount(
        connection,
        taker,
        mintB,
        taker.publicKey
      );
  
      takerAtaB = ata.address;
  
      const metadata_seeds = [
        Buffer.from('metadata'),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
      ];
  
      const master_edition_seeds = [
        ...metadata_seeds,
        Buffer.from("edition")
      ];
  
      metadataB = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
      masterEditionB = PublicKey.findProgramAddressSync(master_edition_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0]; 
    });

    it("Make", async () => {
      const escrow_seeds = [
        Buffer.from('escrow'),
        new PublicKey(maker.publicKey).toBuffer(),
        new PublicKey(mintA).toBuffer(),
        new PublicKey(mintB).toBuffer(),
      ];

      escrow = await PublicKey.findProgramAddressSync(escrow_seeds, programId)[0];

      vault = await getAssociatedTokenAddressSync(
        mintA,
        escrow,
        true
      );

      const signature = await program.methods
      .make(new anchor.BN(1))
      .accounts({
        maker: maker.publicKey,
        makerAta: makerAtaA,
        mintA,
        mintB,
        metadataA,
        masterEditionA,
        makerTokenRecordA: null,
        vaultTokenRecordA: anchor.web3.Keypair.generate().publicKey,
        vault,
        escrow,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker]).rpc({skipPreflight: true}).then(confirm).then(log);
    });
    
    it("Close", async () => {

      const signature = await program.methods
      .close()
      .accounts({
        maker: maker.publicKey,
        makerAta: makerAtaA,
        mintA,
        mintB,
        metadataA,
        masterEditionA,
        makerTokenRecordA: anchor.web3.Keypair.generate().publicKey,
        vaultTokenRecordA: null,
        vault,
        escrow,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker]).rpc({skipPreflight: true}).then(confirm).then(log);
    });
  });

  xdescribe("pNFT escrow", () => {
    
    it("Creates a PNftA", async () => {
      // Metaplex Setup
      const umi = createUmi(connection.rpcEndpoint);
      let umiKeypair = umi.eddsa.createKeypairFromSecretKey(maker.secretKey);
      const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
      umi.use(signerIdentity(signerKeypair));
      umi.use(mplTokenMetadata())
      const mint = generateSigner(umi);
      mintA = new PublicKey(mint.publicKey)
  
      // Create NFT
      let minttx = createProgrammableNft(
        umi, 
        {
          mint: mint,
          authority: signerKeypair,
          updateAuthority: umiKeypair.publicKey,
          name: "NFT Example",
          symbol: "EXM",
          uri: "",
          sellerFeeBasisPoints: percentAmount(0),
          creators: [
              {address: umiKeypair.publicKey, verified: true, share: 100 }
          ],
          collection: null,
          uses: null,
          isMutable: true,
          collectionDetails: null,
        }
      );
  
      const result = await minttx.sendAndConfirm(umi, {
        send: {
          skipPreflight: true
        },
        confirm: {
          commitment
        }
      });
  
      const signature = base58.deserialize(result.signature);
      console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)
  
      // Create Collection Accounts
      let ata = await getOrCreateAssociatedTokenAccount(
        connection,
        maker,
        mintA,
        maker.publicKey
      );
  
      makerAtaA = ata.address;
  
      ata = await getOrCreateAssociatedTokenAccount(
        connection,
        maker,
        mintA,
        taker.publicKey
      );
  
      takerAtaA = ata.address;
  
      const metadata_seeds = [
        Buffer.from('metadata'),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
      ];
  
      const master_edition_seeds = [
        ...metadata_seeds,
        Buffer.from("edition")
      ];
      
      const maker_token_record_seeds = [
        Buffer.from("metadata"),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
        Buffer.from("token_record"),
        new PublicKey(makerAtaA).toBuffer(),
      ];
  
      const taker_token_record_seeds = [
        Buffer.from("metadata"),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
        Buffer.from("token_record"),
        new PublicKey(takerAtaA).toBuffer(),
      ];
  
      metadataA = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
      masterEditionA = PublicKey.findProgramAddressSync(master_edition_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0]; 
      makerTokenRecordA = PublicKey.findProgramAddressSync(maker_token_record_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
      takerTokenRecordA = PublicKey.findProgramAddressSync(taker_token_record_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
    });
  
    it("Creates a PNftB", async () => {
      // Metaplex Setup
      const umi = createUmi(connection.rpcEndpoint);
      let umiKeypair = umi.eddsa.createKeypairFromSecretKey(taker.secretKey);
      const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
      umi.use(signerIdentity(signerKeypair));
      umi.use(mplTokenMetadata())
      const mint = generateSigner(umi);
      mintB = new PublicKey(mint.publicKey)
  
      // Create NFT
      let minttx = createProgrammableNft(
        umi, 
        {
          mint: mint,
          authority: signerKeypair,
          updateAuthority: umiKeypair.publicKey,
          name: "NFT Example",
          symbol: "EXM",
          uri: "",
          sellerFeeBasisPoints: percentAmount(0),
          creators: [
              {address: umiKeypair.publicKey, verified: true, share: 100 }
          ],
          collection: null,
          uses: null,
          isMutable: true,
          collectionDetails: null,
        }
      );
  
      const result = await minttx.sendAndConfirm(umi, {
        send: {
          skipPreflight: true
        },
        confirm: {
          commitment
        }
      });
  
      const signature = base58.deserialize(result.signature);
      console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)
  
      // Create Collection Accounts
      let ata = await getOrCreateAssociatedTokenAccount(
        connection,
        taker,
        mintB,
        maker.publicKey
      );
  
      makerAtaB = ata.address;
  
      ata = await getOrCreateAssociatedTokenAccount(
        connection,
        taker,
        mintB,
        taker.publicKey
      );
  
      takerAtaB = ata.address;
  
      const metadata_seeds = [
        Buffer.from('metadata'),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
      ];
  
      const master_edition_seeds = [
        ...metadata_seeds,
        Buffer.from("edition")
      ];
  
      const maker_token_record_seeds = [
        Buffer.from("metadata"),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
        Buffer.from("token_record"),
        new PublicKey(makerAtaB).toBuffer(),
      ];
  
      const taker_token_record_seeds = [
        Buffer.from("metadata"),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mint.publicKey).toBuffer(),
        Buffer.from("token_record"),
        new PublicKey(takerAtaB).toBuffer(),
      ];
  
      metadataB = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
      masterEditionB = PublicKey.findProgramAddressSync(master_edition_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0]; 
      makerTokenRecordB = PublicKey.findProgramAddressSync(maker_token_record_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
      takerTokenRecordB = PublicKey.findProgramAddressSync(taker_token_record_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];
    });

    it("Make", async () => {
      const escrow_seeds = [
        Buffer.from('escrow'),
        new PublicKey(maker.publicKey).toBuffer(),
        new PublicKey(mintA).toBuffer(),
        new PublicKey(mintB).toBuffer(),
      ];

      escrow = await PublicKey.findProgramAddressSync(escrow_seeds, programId)[0];

      vault = await getAssociatedTokenAddressSync(
        mintA,
        escrow,
        true
      );

      const taker_token_record_seeds = [
        Buffer.from("metadata"),
        new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
        new PublicKey(mintA).toBuffer(),
        Buffer.from("token_record"),
        new PublicKey(vault).toBuffer(),
      ];
      vaultTokenRecordA = PublicKey.findProgramAddressSync(taker_token_record_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];

      const signature = await program.methods
      .make(new anchor.BN(1))
      .accounts({
        maker: maker.publicKey,
        makerAta: makerAtaA,
        mintA,
        mintB,
        metadataA,
        masterEditionA,
        makerTokenRecordA,
        vaultTokenRecordA,
        vault,
        escrow,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker]).rpc({skipPreflight: true}).then(confirm).then(log);
    });
    
    it("Close", async () => {

      const signature = await program.methods
      .close()
      .accounts({
        maker: maker.publicKey,
        makerAta: makerAtaA,
        mintA,
        mintB,
        metadataA,
        masterEditionA,
        makerTokenRecordA,
        vaultTokenRecordA,
        vault,
        escrow,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker]).rpc({skipPreflight: true}).then(confirm).then(log);
    });
  });
    
});
