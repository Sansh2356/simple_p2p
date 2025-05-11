// #![allow(unused)]
// use bitcoin::BlockTime;
// use bitcoin::BlockVersion;
// use bitcoin::CompactTarget;
// use bitcoin::EcdsaSighashType;
// use bitcoin::absolute::Time;
// use bitcoin::consensus::DeserializeError;
// use bitcoin::consensus::Error;
// use bitcoin::consensus::ReadExt;
// use bitcoin::consensus::deserialize;
// use bitcoin::consensus::encode::Decodable;
// use bitcoin::consensus::encode::Encodable;
// use bitcoin::consensus::encode::serialize;
// use bitcoin::ecdsa::Signature;
// use bitcoin::io;
// use bitcoin::io::BufRead;
// use bitcoin::io::Write;
// use bitcoin::p2p::Address as P2P_Address;
// use bitcoin::p2p::ServiceFlags;
// use bitcoin::p2p::address::AddrV2;
// // use bitcoin::secp256k1::PublicKey;
// use bitcoin::PublicKey;
// use bitcoin::TxMerkleNode;
// use bitcoin::transaction::TransactionExt;
// use bitcoin::{Address, BlockHash};
// use bitcoin::{BlockHeader, Transaction};
// use core::str::FromStr;
// use serde::Deserialize;
// use serde::Serialize;
// use std::net::IpAddr;
// use std::net::Ipv4Addr;
// use std::net::SocketAddr;

// use secp256k1::{Secp256k1, SecretKey};
// pub type BeadHash = BlockHash;

// fn main() {
//     #[derive(Clone, Debug, PartialEq, Eq)]

//     pub struct CommittedMetadata {
//         // Committed Braidpool Metadata,
//         pub transaction_cnt: u32,
//         pub transactions: Vec<Transaction>,
//         pub parents: Vec<BeadHash>,
//         pub payout_address: P2P_Address,
//         //timestamp when the bead was created
//         pub observed_time_at_node: Time,
//         pub comm_pub_key: PublicKey,
//         pub miner_ip: AddrV2,
//     }
//     impl Encodable for CommittedMetadata {
//         fn consensus_encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, io::Error> {
//             let mut len = 0;
//             println!("length in committed metadata is --  -- {:?}", len);
//             len += self.transaction_cnt.consensus_encode(w)?;
//             println!("length in committed metadata is --  -- {:?}", len);

//             len += self.transactions.consensus_encode(w)?;
//             println!("length in committed metadata is --  -- {:?}", len);

//             len += self.parents.consensus_encode(w)?;
//             println!("length in committed metadata is --  -- {:?}", len);

//             len += self.payout_address.consensus_encode(w)?;
//             println!("length in committed metadata is --  -- {:?}", len);

//             len += self
//                 .observed_time_at_node
//                 .to_consensus_u32()
//                 .consensus_encode(w)?;
//             println!("length in committed metadata is --  -- {:?}", len);

//             let pubkey_bytes = self.comm_pub_key.to_vec();
//             println!("{:?} PUBKEY BYTES", pubkey_bytes);
//             len += pubkey_bytes.consensus_encode(w)?;
//             println!("length in committed metadata is --  -- {:?}", len);

//             len += self.miner_ip.consensus_encode(w)?;
//             println!("length in committed metadata is --  -- {:?}", len);

//             Ok(len)
//         }
//     }

//     impl Decodable for CommittedMetadata {
//         fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
//             let transaction_cnt = u32::consensus_decode(r)?;
//             let transactions = Vec::<Transaction>::consensus_decode(r)?;
//             let parents = Vec::<BeadHash>::consensus_decode(r)?;
//             let payout_address = P2P_Address::consensus_decode(r)?;
//             let observed_time_at_node =
//                 Time::from_consensus(u32::consensus_decode(r).unwrap()).unwrap();
//             // println!("{:?} PUBLIC KEY",&Vec::<u8>::consensus_decode(r).unwrap());
//             let comm_pub_key =
//                 PublicKey::from_slice(&Vec::<u8>::consensus_decode(r).unwrap()).unwrap();
//             // let secret_key =
//             //     SecretKey::from_byte_array(&[0xcd; 32]).expect("32 bytes, within curve order");
//             // let comm_pub_key = PublicKey::from_secret_key(&secp, &secret_key);
//             // let comm_pub_key =
//             //     PublicKey::from_slice(&Vec::<u8>::consensus_decode(r).unwrap()).unwrap();

//             let miner_ip = AddrV2::consensus_decode(r)?;
//             Ok(CommittedMetadata {
//                 transaction_cnt,
//                 transactions,
//                 parents,
//                 payout_address,
//                 observed_time_at_node,
//                 comm_pub_key,
//                 miner_ip,
//             })
//         }
//     }
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub struct TimeVec(pub Vec<Time>);
//     impl Encodable for TimeVec {
//         fn consensus_encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, io::Error> {
//             let mut len = 0;
//             // Encode the length for deterministic encoding
//             len += (self.0.len() as u64).consensus_encode(w)?;
//             for time in &self.0 {
//                 len += time.to_consensus_u32().consensus_encode(w)?;
//             }
//             Ok(len)
//         }
//     }

//     impl Decodable for TimeVec {
//         fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
//             let len = u64::consensus_decode(r)?;
//             let mut vec = Vec::with_capacity(len as usize);
//             for _ in 0..len {
//                 let time_u32 = u32::consensus_decode(r)?;
//                 let time = Time::from_consensus(time_u32).unwrap();
//                 vec.push(time);
//             }
//             Ok(TimeVec(vec))
//         }
//     }
//     let test_sock_add = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8888);
//     let _address = P2P_Address::new(&test_sock_add.clone(), ServiceFlags::NONE);
//     // let secp = Secp256k1::new();
//     // let secret_key = SecretKey::from_byte_array(&[0xcd; 32]).expect("32 bytes, within curve order");
//     // let public_key = PublicKey::from_secret_key(&secp, &secret_key);
//     let public_key = "020202020202020202020202020202020202020202020202020202020202020202"
//         .parse::<bitcoin::PublicKey>()
//         .unwrap();
//     // println!("{:?} ", public_key.serialize());

//     let socket = bitcoin::p2p::address::AddrV2::Ipv4(Ipv4Addr::new(127, 0, 0, 1));
//     let time_hash_set = TimeVec(Vec::new());
//     let parent_hash_set = Vec::new();
//     // let time_val: u32 = 25;
//     // let t = Time::from_consensus(time_val).expect("Invalid time format");
//     let t: u32 = 1653195600; // May 22nd, 5am UTC.
//     let time = Time::from_consensus(t).expect("invalid time value");

//     let test_committed_metadata = CommittedMetadata {
//         transaction_cnt: 24,
//         parents: parent_hash_set,
//         transactions: vec![],
//         payout_address: _address,
//         comm_pub_key: public_key,
//         observed_time_at_node: time,
//         miner_ip: socket,
//     };

//     // let referenced_metadata = test_committed_metadata.clone();
//     // let serialized_val = serialize(&referenced_metadata);
//     // println!(
//     //     "the serialized committed metadata is this value --- {:?}  {:?} ",
//     //     serialized_val,
//     //     serialized_val.len()
//     // );
//     // let deserialized_result: Result<CommittedMetadata, DeserializeError> =
//     //     deserialize(&serialized_val);
//     // let desesrialized_test_bead = match deserialized_result {
//     //     Ok(val) => val,
//     //     Err(error) => {
//     //         panic!(
//     //             "An error occurred while deserializaing committed metadata {:?}",
//     //             error
//     //         );
//     //     }
//     // };
//     // println!(
//     //     "the deserialized value of committed metadata is ---- {:?} ",
//     //     desesrialized_test_bead
//     // );
//     //signature//
//     let hex = "3046022100839c1fbc5304de944f697c9f4b1d01d1faeba32d751c0f7acb21ac8a0f436a72022100e89bd46bb3a5a62adc679f659b7ce876d83ee297c7a5587b2011c4fcc72eab45";
//     let sig = Signature {
//         signature: secp256k1::ecdsa::Signature::from_str(hex).unwrap(),
//         sighash_type: EcdsaSighashType::All,
//     };
//     #[derive(Clone, Debug, PartialEq, Eq)]

//     pub struct UnCommittedMetadata {
//         //Uncomitted Metadata
//         //timestamp when the bead was broadcasted
//         pub extra_nonce: i32,
//         pub broadcast_timestamp: Time,
//         pub signature: Signature,
//         pub parent_bead_timestamps: TimeVec,
//     }
//     impl Encodable for UnCommittedMetadata {
//         fn consensus_encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, io::Error> {
//             let mut len = 0;
//             len += self.extra_nonce.consensus_encode(w)?;
//             len += self
//                 .broadcast_timestamp
//                 .to_consensus_u32()
//                 .consensus_encode(w)?;
//             len += self.signature.to_string().consensus_encode(w)?;
//             len += self.parent_bead_timestamps.consensus_encode(w)?;
//             Ok(len)
//         }
//     }

//     impl Decodable for UnCommittedMetadata {
//         fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
//             let extra_nonce = i32::consensus_decode(r)?;
//             let broadcast_timestamp =
//                 Time::from_consensus(u32::consensus_decode(r).unwrap()).unwrap();
//             let signature = Signature::from_str(&String::consensus_decode(r).unwrap()).unwrap();
//             let parent_bead_timestamps = TimeVec::consensus_decode(r)?;
//             Ok(UnCommittedMetadata {
//                 extra_nonce,
//                 broadcast_timestamp,
//                 signature,
//                 parent_bead_timestamps,
//             })
//         }
//     }
//     let test_uncommittedmetadata: UnCommittedMetadata = UnCommittedMetadata {
//         extra_nonce: 12,
//         broadcast_timestamp: time,
//         signature: sig,
//         parent_bead_timestamps: time_hash_set,
//     };
//     // let serialized_val = serialize(&test_uncommittedmetadata);
//     // println!(
//     //     "the serialized uncommitted metadata is this value --- {:?}  {:?} ",
//     //     serialized_val,
//     //     serialized_val.len()
//     // );
//     // let deserialized_result: Result<UnCommittedMetadata, DeserializeError> =
//     //     deserialize(&serialized_val);
//     // let desesrialized_test_bead = match deserialized_result {
//     //     Ok(val) => val,
//     //     Err(error) => {
//     //         panic!(
//     //             "An error occurred while deserializaing uncommitted metadata {:?}",
//     //             error
//     //         );
//     //     }
//     // };
//     // println!(
//     //     "the deserialized value of uncommitted metadata is ---- {:?} ",
//     //     desesrialized_test_bead
//     // );
//     #[derive(Clone, Debug)]

//     pub struct Bead {
//         pub block_header: BlockHeader,
//         pub committed_metadata: CommittedMetadata,
//         pub uncommitted_metadata: UnCommittedMetadata,
//     }
//     let test_bytes: [u8; 32] = [0u8; 32];

//     let test_bead = Bead {
//         block_header: BlockHeader {
//             version: BlockVersion::TWO,
//             prev_blockhash: BlockHash::from_byte_array(test_bytes),
//             bits: CompactTarget::from_consensus(32),
//             nonce: 1,
//             time: BlockTime::from_u32(8328429),
//             merkle_root: TxMerkleNode::from_byte_array(test_bytes),
//         },
//         committed_metadata: test_committed_metadata,
//         uncommitted_metadata: test_uncommittedmetadata,
//     };
//     impl Encodable for Bead {
//         fn consensus_encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, io::Error> {
//             let mut len = 0;
//             len += self.block_header.consensus_encode(w)?;
//             println!("len is {:?} ", len);
//             len += self.committed_metadata.consensus_encode(w)?;
//             println!("len is {:?} ", len);

//             len += self.uncommitted_metadata.consensus_encode(w)?;
//             println!("len is {:?} ", len);

//             Ok(len)
//         }
//     }

//     impl Decodable for Bead {
//         fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
//             let block_header = BlockHeader::consensus_decode(r)?;
//             let committed_metadata = CommittedMetadata::consensus_decode(r)?;
//             let uncommitted_metadata = UnCommittedMetadata::consensus_decode(r)?;
//             Ok(Bead {
//                 block_header,
//                 committed_metadata,
//                 uncommitted_metadata,
//             })
//         }
//     }
//     let serialized_val = serialize(&test_bead);
//     println!(
//         "the serialized bead is this value --- {:?}  {:?} ",
//         serialized_val,
//         serialized_val.len()
//     );
//     let deserialized_result: Result<Bead, DeserializeError> = deserialize(&serialized_val);
//     let desesrialized_test_bead = match deserialized_result {
//         Ok(val) => val,
//         Err(error) => {
//             panic!(
//                 "An error occurred while deserializaing bead metadata {:?}",
//                 error
//             );
//         }
//     };
//     println!(
//         "the deserialized value of bead is ---- {:?} ",
//         desesrialized_test_bead
//     );
// }

pub mod client;
pub mod server;
use client::client_rpc::{World, WorldClient};
use futures::prelude::*;
use server::server_rpc::HelloServer;
use tarpc::{
    client as tarpc_client, context,
    server::{self as tarpc_server, Channel},
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (client_transport, server_transport) = tarpc::transport::channel::unbounded();
    HelloServer.serve();
    let server_tarpc = tarpc_server::BaseChannel::with_defaults(server_transport);
    tokio::spawn(
        server_tarpc
            .execute(HelloServer.serve())
            // Handle all requests concurrently.
            .for_each(|response| async move {
                tokio::spawn(response);
            }),
    );

    // WorldClient is generated by the #[tarpc::service] attribute. It has a constructor `new`
    // that takes a config and any Transport as input.
    let client_tarpc = WorldClient::new(tarpc_client::Config::default(), client_transport).spawn();

    // The client has an RPC method for each RPC defined in the annotated trait. It takes the same
    // args as defined, with the addition of a Context, which is always the first arg. The Context
    // specifies a deadline and trace information which can be helpful in debugging requests.
    let hello = client_tarpc
        .hello(context::current(), "Stim".to_string())
        .await?;

    println!("{hello}");

    Ok(())
}
