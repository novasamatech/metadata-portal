mod generate;
mod github;
pub(crate) mod source;
mod wasm;

use std::collections::{BTreeMap, HashMap};
use std::process::exit;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use blake2_rfc::blake2b::blake2b;
use log::{info, warn};
use rayon::prelude::*;
use sp_core::H256;

use crate::common::path::QrPath;
use crate::common::types::get_crypto;
use crate::config::{AppConfig, Chain};
use crate::fetch::Fetcher;
use crate::qrs::{metadata_files, spec_files};
use crate::source::{save_source_info, Source};
use crate::updater::generate::{download_metadata_qr, generate_metadata_qr, generate_spec_qr};
use crate::updater::github::fetch_latest_runtime;
use crate::updater::wasm::{download_wasm, meta_values_from_wasm_bytes};

pub(crate) fn update_from_node<F>(
    config: AppConfig,
    sign: bool,
    signing_key: String,
    fetcher: F,
) -> anyhow::Result<()>
where
    F: Fetcher + Sync,
{
    let metadata_qrs = metadata_files(&config.qr_dir)?;
    let specs_qrs = spec_files(&config.qr_dir)?;
    let is_changed = AtomicBool::new(false);
    let error_fetching_data = AtomicBool::new(false);

    config
        .chains
        .par_iter()
        .try_for_each(|chain| -> anyhow::Result<()> {
            let changed = process_chain(
                chain,
                &config,
                &fetcher,
                &metadata_qrs,
                &specs_qrs,
                sign,
                &signing_key,
                &error_fetching_data,
            )?;
            if changed {
                is_changed.store(true, Ordering::Relaxed);
            }
            Ok(())
        })?;

    if error_fetching_data.load(Ordering::Relaxed) {
        warn!("⚠️ Some chain data wasn't read. Please check the log!");
        exit(12);
    }

    if !is_changed.load(Ordering::Relaxed) {
        info!("🎉 Everything is up to date!");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_chain<F>(
    chain: &Chain,
    config: &AppConfig,
    fetcher: &F,
    metadata_qrs: &HashMap<String, BTreeMap<u32, QrPath>>,
    specs_qrs: &HashMap<String, QrPath>,
    sign: bool,
    signing_key: &str,
    error_fetching_data: &AtomicBool,
) -> anyhow::Result<bool>
where
    F: Fetcher + Sync,
{
    let encryption = get_crypto(chain);
    let mut is_changed = false;

    if !specs_qrs.contains_key(&chain.portal_id()) {
        match fetcher.fetch_specs(chain) {
            Ok(specs) => {
                if chain.verifier == "parity" {
                    warn!("The chain {} should be added and signed by Parity, please check it on the Parity Metadata portal https://metadata.parity.io/", chain.name);
                } else {
                    generate_spec_qr(
                        &specs,
                        &config.qr_dir,
                        &chain.portal_id(),
                        sign,
                        signing_key.to_owned(),
                        &encryption,
                    )?;
                }
                is_changed = true;
            }
            Err(e) => {
                error_fetching_data.store(true, Ordering::Relaxed);
                warn!("Can't get specs for {}. Error is {}", chain.name, e);
                return Ok(is_changed);
            }
        }
    }

    let fetched_meta = match fetcher.fetch_metadata(chain) {
        Ok(meta) => meta,
        Err(e) => {
            error_fetching_data.store(true, Ordering::Relaxed);
            warn!("Can't get metadata for {}. Error is {}", chain.name, e);
            return Ok(is_changed);
        }
    };

    let version = fetched_meta.meta_values.version;

    // Skip if already have QR for the same version
    if let Some(map) = metadata_qrs.get(&chain.portal_id()) {
        if map.contains_key(&version) {
            return Ok(is_changed);
        }
    }

    if chain.verifier == "parity" {
        download_metadata_qr(
            "https://github.com/paritytech/metadata-portal/raw/master/public/qr",
            &fetched_meta.meta_values,
            &config.qr_dir,
        )?;
    } else {
        let path = generate_metadata_qr(
            &fetched_meta.meta_values,
            &fetched_meta.genesis_hash,
            &config.qr_dir,
            sign,
            signing_key.to_owned(),
            &encryption,
            &chain.portal_id(),
        )?;
        let source = Source::Rpc {
            block: fetched_meta.block_hash,
        };
        save_source_info(&path, &source)?;
    }

    Ok(true)
}

#[tokio::main]
pub(crate) async fn update_from_github(
    config: AppConfig,
    sign: bool,
    signing_key: String,
) -> anyhow::Result<()> {
    let metadata_qrs = metadata_files(&config.qr_dir)?;
    for chain in config.chains {
        info!("🔍 Checking for updates for {}", chain.name);
        if chain.github_release.is_none() {
            info!("↪️ No GitHub releases configured, skipping",);
            continue;
        }

        let github_repo = chain.github_release.as_ref().unwrap();
        let wasm = fetch_latest_runtime(github_repo, &chain.name).await?;
        if wasm.is_none() {
            warn!("🤨 No releases found");
            continue;
        }
        let wasm = wasm.unwrap();
        info!("📅 Found version {}", wasm.version);
        let genesis_hash = H256::from_str(&github_repo.genesis_hash).unwrap();

        // Skip if already have QR for the same version
        if let Some(map) = metadata_qrs.get(&chain.portal_id()) {
            if map.contains_key(&wasm.version) || map.keys().min().unwrap_or(&0) > &wasm.version {
                info!("🎉 {} is up to date!", chain.name);
                continue;
            }
        }
        let wasm_bytes = download_wasm(wasm.to_owned()).await?;
        let meta_hash = blake2b(32, &[], &wasm_bytes).as_bytes().to_vec();
        let meta_values = meta_values_from_wasm_bytes(&wasm_bytes)?;
        let encryption = get_crypto(&chain);
        let path = generate_metadata_qr(
            &meta_values,
            &genesis_hash,
            &config.qr_dir,
            sign,
            signing_key.to_owned(),
            &encryption,
            &chain.portal_id(),
        )?;
        let source = Source::Wasm {
            github_repo: format!("{}/{}", github_repo.owner, github_repo.repo),
            hash: format!("0x{}", hex::encode(meta_hash)),
        };
        save_source_info(&path, &source)?;
    }
    Ok(())
}
