use std::fmt::Error;

use bitcoin::block::BlockHash;
use bitcoin::consensus::encode::deserialize as deserialize_2;
use bitcoin::consensus::{DeserializeError, deserialize as deserialize_1, serialize};

use bitcoin::sign_message::MessageSignature;
use bitcoin::{Block, BlockHeader, BlockTime, BlockVersion, CompactTarget, TxMerkleNode};
pub fn dummy_test() {
    let mut test_bytes = [0u8; 32];
    let mut temp_header = BlockHeader {
        version: BlockVersion::TWO,
        prev_blockhash: BlockHash::from_byte_array(test_bytes),
        bits: CompactTarget::from_consensus(32),
        nonce: 1,
        time: BlockTime::from_u32(8328429),
        merkle_root: TxMerkleNode::from_byte_array(test_bytes),
    };
    let serialize_result = serialize(&temp_header.clone());

    println!(
        "serialized using rust bitcoin serialize trait {:?} FIRST ONE -----",
        serialize_result
    );
    let mut deserialized_result: Result<BlockHeader, DeserializeError> =
        deserialize_1(&serialize_result);
    let deserialized_header: BlockHeader = match deserialized_result {
        Ok(header) => header,
        Err(error) => {
            panic!("An error occurred while deserialzing the block {:?}", error);
        }
    };
    let segwit = include_bytes!(
        "../testnet_block_000000000000045e0b1660b6445b5e5c5ab63c9a4f956be7e1e69be04fa4497b.raw"
    );

    let block: Block = deserialize_2(segwit).unwrap();
    println!(
        "deserialized using rust bitcoin deserialize trait {:?} FIRST ONE -----",
        deserialized_header
    );
    // println!("{:?} --SECOND ONE----", block);
}
