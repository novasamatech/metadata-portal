use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

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

/// How many times the whole endpoint list is tried before giving up.
const FETCH_ROUNDS: u32 = 2;
/// Pause before the endpoint list is tried again, so a short outage can pass.
const FETCH_RETRY_DELAY: Duration = Duration::from_secs(2);

/// Calls `f` with each URL until one succeeds.
///
/// A dead node is skipped right away instead of being retried in place, so it
/// doesn't hold up the chain while other nodes are healthy. Only when every
/// node failed is the list tried again: many chains have a single endpoint,
/// and one dropped connection shouldn't fail the whole run.
fn call_urls<F, T>(urls: &[String], f: F) -> Result<T>
where
    F: Fn(&str) -> Result<T, generate_message::Error>,
{
    call_urls_with_retry(urls, f, FETCH_ROUNDS, FETCH_RETRY_DELAY)
}

fn call_urls_with_retry<F, T>(
    urls: &[String],
    f: F,
    rounds: u32,
    retry_delay: Duration,
) -> Result<T>
where
    F: Fn(&str) -> Result<T, generate_message::Error>,
{
    let mut last_err: Option<generate_message::Error> = None;
    for round in 0..rounds {
        if round > 0 {
            thread::sleep(retry_delay);
        }
        for url in urls.iter() {
            match f(url) {
                Ok(res) => return Ok(res),
                Err(e) => {
                    warn!("Failed to fetch {}: {:?}", url, e);
                    last_err = Some(e);
                }
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    const ROUNDS: u32 = 2;
    const NO_DELAY: Duration = Duration::ZERO;

    fn urls(list: &[&str]) -> Vec<String> {
        list.iter().map(|url| url.to_string()).collect()
    }

    #[test]
    fn skips_dead_node_without_retrying_it() {
        let calls = RefCell::new(Vec::new());

        let result = call_urls_with_retry(
            &urls(&["dead", "alive"]),
            |url| {
                calls.borrow_mut().push(url.to_string());
                match url {
                    "alive" => Ok(url.to_string()),
                    _ => Err(generate_message::Error::NotSupported),
                }
            },
            ROUNDS,
            NO_DELAY,
        );

        assert_eq!(result.unwrap(), "alive");
        assert_eq!(*calls.borrow(), ["dead", "alive"]);
    }

    #[test]
    fn retries_the_list_when_every_node_failed() {
        let calls = RefCell::new(0);

        let result = call_urls_with_retry(
            &urls(&["flaky"]),
            |url| {
                *calls.borrow_mut() += 1;
                if *calls.borrow() < ROUNDS {
                    Err(generate_message::Error::NotSupported)
                } else {
                    Ok(url.to_string())
                }
            },
            ROUNDS,
            NO_DELAY,
        );

        assert_eq!(result.unwrap(), "flaky");
        assert_eq!(*calls.borrow(), ROUNDS);
    }

    #[test]
    fn fails_after_all_rounds() {
        let calls = RefCell::new(0);

        let result: Result<()> = call_urls_with_retry(
            &urls(&["dead"]),
            |_| {
                *calls.borrow_mut() += 1;
                Err(generate_message::Error::NotSupported)
            },
            ROUNDS,
            NO_DELAY,
        );

        assert!(result.is_err());
        assert_eq!(*calls.borrow(), ROUNDS);
    }

    #[test]
    fn fails_without_urls() {
        let result: Result<()> = call_urls_with_retry(&[], |_| Ok(()), ROUNDS, NO_DELAY);

        assert!(result.is_err());
    }
}
