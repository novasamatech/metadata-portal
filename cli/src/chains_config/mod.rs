use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use log::warn;
use serde::{Deserialize, Serialize};

use crate::config::{Chain, Verifier};
use crate::fetch::{ConfigRpcFetcher, Fetcher};
use crate::opts::ChainsOpts;
use crate::AppConfig;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct ConfigTemplate {
    pub(crate) data_file: Option<PathBuf>,
    pub(crate) public_dir: PathBuf,
    pub(crate) verifiers: HashMap<String, Verifier>,
    pub(crate) chains: HashMap<String, ChainTemplate>,
    pub(crate) inject_chains: Vec<InjectedChain>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct ChainTemplate {
    pub(crate) name: String,
    pub(crate) color: String,
    pub(crate) verifier: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct InjectedChain {
    pub(crate) name: String,
    pub(crate) title: String,
    pub(crate) color: String,
    pub(crate) verifier: String,
    pub(crate) icon: String,
    pub(crate) rpc_endpoints: Vec<String>,
    pub(crate) testnet: bool,
    pub(crate) encryption: Option<String>,
    pub(crate) relay_chain: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChainJSON {
    pub(crate) name: String,
    pub(crate) nodes: Vec<ChainNode>,
    pub(crate) icon: String,
    pub(crate) options: Option<Vec<String>>,
    pub(crate) parent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ChainNode {
    pub(crate) name: String,
    pub(crate) url: String,
}

const EXCLUDE_CHAINS: [&str; 4] = [
    "Arctic Relay Testnet",
    "Aleph Zero Testnet", //TODO name matches with mainnet and will override it
    "Hashed Network",     // Specs(Base58PrefixMismatch { specs: 9072, meta: 42 })
    "Cere",               //TODO spece name is `node` that is the same for the Polkadex
];

/// Chains of the config generated last time, keyed by title.
///
/// Empty when there is no readable config yet.
fn previous_chains_by_title(config_path: &Path) -> HashMap<String, Chain> {
    match AppConfig::load(config_path) {
        Ok(config) => config
            .chains
            .into_iter()
            .filter_map(|chain| chain.title.clone().map(|title| (title, chain)))
            .collect(),
        Err(e) => {
            warn!(
                "Can't read previous {}, chains with unreachable nodes will be skipped. {}",
                config_path.display(),
                e
            );
            HashMap::new()
        }
    }
}

/// Adds `chain` unless a chain with the same portal id is already listed.
///
/// Returns whether the chain was added.
fn push_unique(chains: &mut Vec<Chain>, chain: Chain) -> bool {
    let portal_id = chain.portal_id().to_lowercase();
    if chains
        .iter()
        .any(|listed| listed.portal_id().to_lowercase() == portal_id)
    {
        return false;
    }
    chains.push(chain);
    true
}

pub(crate) fn update_chains_config(chains_opts: ChainsOpts) -> Result<()> {
    let template_path = Path::new("config-template.toml");
    let config_template_toml = fs::read_to_string(template_path)?;
    let config_template = toml::from_str::<ConfigTemplate>(config_template_toml.as_str())?;
    let mut relay_chains: HashMap<String, String> = HashMap::new(); //TODO make a constant
    relay_chains.insert(
        String::from("91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3"),
        String::from("polkadot"),
    );
    relay_chains.insert(
        String::from("b0a8d493285c2df73290dfb7e61f870f17b41801197a149ca93654499ea3dafe"),
        String::from("kusama"),
    );
    relay_chains.insert(
        String::from("6408de7737c59c238890533af25896a2c20608d8b380bb01029acb392781063e"),
        String::from("rococo"),
    );
    relay_chains.insert(
        String::from("e143f23803ac50e8f6f8e62695d1ce9e4e1d68aa36c1cd2cfd15340213f3423e"),
        String::from("westend"),
    );

    let chain_params = match chains_opts.env.as_str() {
        "dev" => (
            "config_dev.toml",
            "public/data_dev.json",
            format!(
                "https://raw.githubusercontent.com/novasamatech/nova-utils/master/chains/{}/{}",
                chains_opts.version, "chains_dev.json"
            ),
            "public/qr_dev",
        ),
        "prod" => (
            "config.toml",
            "public/data.json",
            format!(
                "https://raw.githubusercontent.com/novasamatech/nova-utils/master/chains/{}/{}",
                chains_opts.version, "chains.json"
            ),
            "public/qr",
        ),
        _ => bail!("Unknown env. Should be dev or prod"),
    };

    let previous_chains = previous_chains_by_title(Path::new(chain_params.0));

    let chains_json_response = reqwest::blocking::get(chain_params.2).unwrap();
    let chains_json: Vec<ChainJSON> = chains_json_response.json().unwrap();

    let mut chains: Vec<Chain> = vec![];
    for chain in chains_json {
        if EXCLUDE_CHAINS.contains(&chain.name.as_str()) {
            continue;
        }
        if chain.options.is_some()
            && chain
                .options
                .clone()
                .unwrap()
                .contains(&String::from("noSubstrateRuntime"))
        {
            continue;
        }

        let chain_template = config_template.chains.get(&chain.name);
        match chain_template {
            Some(chain_template) => {
                chains.push(Chain {
                    name: String::from(&chain_template.name),
                    title: Some(chain.name),
                    color: String::from(&chain_template.color),
                    icon: chain.icon,
                    rpc_endpoints: chain.nodes.iter().map(|node| node.url.clone()).collect(),
                    github_release: None,
                    token_decimals: None,
                    token_unit: None,
                    testnet: match &chain.options {
                        Some(options) => Some(options.contains(&String::from("testnet"))),
                        None => Some(false),
                    },
                    verifier: match &chain_template.verifier {
                        Some(value) => String::from(value),
                        None => String::from("novasama"),
                    },
                    encryption: match &chain.options {
                        Some(options) => {
                            if options.contains(&String::from("ethereumBased")) {
                                Some(String::from("ethereum"))
                            } else {
                                None
                            }
                        }
                        None => None,
                    },
                    relay_chain: chain
                        .parent_id
                        .map(|parent_id| String::from(relay_chains.get(&parent_id).unwrap())),
                });
            }
            None => {
                eprintln!(
                    "No chain {} found in config template, getting data from chain",
                    chain.name
                );
                let dummy_chain = Chain {
                    name: chain.name.clone(),
                    title: Some(chain.name.clone()),
                    color: String::from("#000000"),
                    icon: chain.icon.clone(),
                    rpc_endpoints: chain.nodes.iter().map(|node| node.url.clone()).collect(),
                    token_unit: None,
                    token_decimals: None,
                    github_release: None,
                    testnet: match &chain.options {
                        Some(options) => Some(options.contains(&String::from("testnet"))),
                        None => Some(false),
                    },
                    verifier: String::from("novasama"),
                    encryption: match &chain.options {
                        Some(options) => {
                            if options.contains(&String::from("ethereumBased")) {
                                Some(String::from("ethereum"))
                            } else {
                                None
                            }
                        }
                        None => None,
                    },
                    relay_chain: chain
                        .parent_id
                        .as_ref()
                        .map(|parent_id| String::from(relay_chains.get(parent_id).unwrap())),
                };
                let fetcher = ConfigRpcFetcher;
                let fetch_result = fetcher.fetch_specs(&dummy_chain);
                let chain_specs = match fetch_result {
                    Ok(res) => res,
                    Err(e) => {
                        // An unreachable node must not drop a chain that is already
                        // served: the cleaner would delete its QRs afterwards.
                        match previous_chains.get(&chain.name) {
                            Some(previous) => {
                                warn!(
                                    "Error getting network {} chainspec, keep its previous config. {}",
                                    dummy_chain.name, e
                                );
                                chains.push(Chain {
                                    icon: dummy_chain.icon,
                                    rpc_endpoints: dummy_chain.rpc_endpoints,
                                    ..previous.clone()
                                });
                            }
                            None => warn!(
                                "Error getting network {} chainspec, skip it. {}",
                                dummy_chain.name, e
                            ),
                        }
                        continue;
                    }
                };

                chains.push(Chain {
                    name: chain_specs.name,
                    title: Some(chain.name.clone()),
                    color: String::from("#000000"),
                    icon: chain.icon.clone(),
                    rpc_endpoints: chain.nodes.iter().map(|node| node.url.clone()).collect(),
                    github_release: None,
                    token_decimals: None,
                    token_unit: None,
                    testnet: match &chain.options {
                        Some(options) => Some(options.contains(&String::from("testnet"))),
                        None => Some(false),
                    },
                    verifier: String::from("novasama"),
                    encryption: match &chain.options {
                        Some(options) => {
                            if options.contains(&String::from("ethereumBased")) {
                                Some(String::from("ethereum"))
                            } else {
                                None
                            }
                        }
                        None => None,
                    },
                    relay_chain: chain
                        .parent_id
                        .as_ref()
                        .map(|parent_id| String::from(relay_chains.get(parent_id).unwrap())),
                });
                warn!(
                    "Add chain {} in config-template.toml in order to speed up next chain update",
                    chain.name
                );
            }
        }
    }
    for chain in config_template.inject_chains {
        let injected = Chain {
            name: chain.name.clone(),
            title: Some(chain.title.clone()),
            color: chain.color,
            icon: chain.icon.clone(),
            rpc_endpoints: chain.rpc_endpoints,
            github_release: None,
            token_decimals: None,
            token_unit: None,
            testnet: Some(chain.testnet),
            verifier: chain.verifier,
            encryption: chain.encryption.as_ref().map(String::from),
            relay_chain: chain.relay_chain.clone(),
        };
        if !push_unique(&mut chains, injected) {
            warn!(
                "Injected chain {} is already in the chains list, remove it from config-template.toml",
                chain.name
            );
        }
    }

    let new_config = AppConfig {
        data_file: PathBuf::from(chain_params.1),
        public_dir: config_template.public_dir,
        qr_dir: PathBuf::from(chain_params.3),
        verifiers: config_template.verifiers,
        chains,
    };
    let saved = new_config.save(Path::new(chain_params.0));
    if saved.is_err() {
        return Err(saved.err().unwrap());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_unique_skips_listed_portal_id() {
        let mut chains = vec![Chain::default()];

        let added = push_unique(
            &mut chains,
            Chain {
                name: "Polkadot".to_string(),
                title: Some("Injected".to_string()),
                ..Chain::default()
            },
        );

        assert!(!added);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].title, None);
    }

    #[test]
    fn push_unique_adds_same_name_on_another_relay_chain() {
        let mut chains = vec![Chain::default()];

        let added = push_unique(
            &mut chains,
            Chain {
                relay_chain: Some("kusama".to_string()),
                ..Chain::default()
            },
        );

        assert!(added);
        assert_eq!(chains.len(), 2);
    }
}
