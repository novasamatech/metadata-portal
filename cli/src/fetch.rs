use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use definitions::network_specs::NetworkSpecs;
use generate_message::helpers::{meta_fetch, specs_agnostic, MetaFetched};
use generate_message::parser::Token;
use log::warn;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::common::types::get_crypto;
use crate::config::{AppConfig, Chain};
use crate::export::{ExportData, ReactAssetPath};

pub(crate) trait Fetcher {
    fn fetch_specs(&self, chain: &Chain) -> Result<NetworkSpecs>;
    fn fetch_metadata(&self, chain: &Chain) -> Result<MetaFetched>;
}

// Try each URL once; move on quickly when a node is unreachable so a single
// dead endpoint doesn't hold up the whole chain.
fn call_urls<F, T>(urls: &[String], f: F) -> Result<T>
where
    F: Fn(&str) -> Result<T, generate_message::Error>,
{
    let mut last_err: Option<generate_message::Error> = None;
    for url in urls.iter() {
        match f(url) {
            Ok(res) => return Ok(res),
            Err(e) => {
                warn!("Failed to fetch {}: {:?}", url, e);
                last_err = Some(e);
            }
        }
    }
    match last_err {
        Some(e) => bail!("Error calling chain node: {:?}", e),
        None => bail!("Error calling chain node: no urls configured"),
    }
}

pub(crate) struct RpcFetcher;

impl Fetcher for RpcFetcher {
    fn fetch_specs(&self, chain: &Chain) -> Result<NetworkSpecs> {
        let specs = call_urls(&chain.rpc_endpoints, |url| {
            let optional_token_override = chain.token_decimals.zip(chain.token_unit.as_ref()).map(
                |(token_decimals, token_unit)| Token {
                    decimals: token_decimals,
                    unit: token_unit.to_string(),
                },
            );

            specs_agnostic(url, get_crypto(chain), optional_token_override, None)
        })
        .map_err(|e| anyhow!("{:?}", e))?;
        if specs.name.to_lowercase() != chain.name {
            bail!(
                "Network name mismatch. Expected {}, got {}. Please fix it in `config.toml`",
                chain.name,
                specs.name
            )
        }
        Ok(specs)
    }

    fn fetch_metadata(&self, chain: &Chain) -> Result<MetaFetched> {
        let meta = call_urls(&chain.rpc_endpoints, meta_fetch).map_err(|e| anyhow!("{:?}", e))?;
        if meta.meta_values.name.to_lowercase() != chain.name {
            bail!(
                "Network name mismatch. Expected {}, got {}. Please fix it in `config.toml`",
                chain.name,
                meta.meta_values.name
            )
        }
        Ok(meta)
    }
}

pub(crate) struct ConfigRpcFetcher;

impl Fetcher for ConfigRpcFetcher {
    fn fetch_specs(&self, chain: &Chain) -> Result<NetworkSpecs> {
        let specs = call_urls(&chain.rpc_endpoints, |url| {
            let optional_token_override = chain.token_decimals.zip(chain.token_unit.as_ref()).map(
                |(token_decimals, token_unit)| Token {
                    decimals: token_decimals,
                    unit: token_unit.to_string(),
                },
            );

            specs_agnostic(url, get_crypto(chain), optional_token_override, None)
        })
        .map_err(|e| anyhow!("{:?}", e))?;
        Ok(specs)
    }

    fn fetch_metadata(&self, _chain: &Chain) -> Result<MetaFetched> {
        bail!("Not implemented!");
    }
}

#[derive(Serialize, Deserialize)]
struct PkgJson {
    homepage: String,
}
pub(crate) fn fetch_deployed_data(config: &AppConfig) -> Result<ExportData> {
    let pkg_json = fs::read_to_string(Path::new("package.json"))?;
    let pkg_json: PkgJson = serde_json::from_str(&pkg_json)?;

    let data_file = ReactAssetPath::from_fs_path(&config.data_file, &config.public_dir)?;
    let url = Url::parse(&pkg_json.homepage)?;
    let url = url.join(&data_file.to_string())?;

    Ok(reqwest::blocking::get(url)?.json::<ExportData>()?)
}
