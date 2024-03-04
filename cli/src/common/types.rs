// Name that uniquely identifies the chain. For example there could be a `statemine` on Kusama and Rococo.
pub(crate) type ChainPortalId = String;
pub(crate) type MetaVersion = u32;

use definitions::crypto::Encryption;

use crate::config::Chain;

pub(crate) fn get_crypto(chain: &Chain) -> Encryption {
    match &chain.encryption {
        Some(encryption) => {
            if encryption == "ethereum" {
                Encryption::Ethereum
            } else if encryption == "ecdsa" {
                Encryption::Ecdsa
            } else {
                Encryption::Sr25519
            }
        }
        _ => Encryption::Sr25519,
    }
}
