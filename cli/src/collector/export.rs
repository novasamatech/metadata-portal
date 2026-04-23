use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use log::{info, warn};
use rayon::prelude::*;

use crate::common::path::{ContentType, QrPath};
use crate::common::types::MetaVersion;
use crate::config::Chain;
use crate::export::{ExportChainSpec, ExportData, MetadataQr, QrCode, ReactAssetPath};
use crate::fetch::{fetch_deployed_data, Fetcher};
use crate::qrs::{collect_metadata_qrs, metadata_files, spec_files};
use crate::AppConfig;

pub(crate) fn export_specs<F>(config: &AppConfig, fetcher: F) -> Result<ExportData>
where
    F: Fetcher + Sync,
{
    let all_specs = spec_files(&config.qr_dir)?;
    let all_metadata = metadata_files(&config.qr_dir)?;
    let online = fetch_deployed_data(config).ok();

    let chain_specs: Vec<Result<(String, ExportChainSpec)>> = config
        .chains
        .par_iter()
        .map(|chain| {
            info!("Collecting {} info...", chain.name);
            build_chain_spec(chain, config, &fetcher, &all_specs, &all_metadata, online.as_ref())
        })
        .collect();

    let mut export_specs = IndexMap::new();
    for result in chain_specs {
        let (portal_id, chain_spec) = result?;
        export_specs.insert(portal_id, chain_spec);
    }
    Ok(export_specs)
}

fn build_chain_spec<F>(
    chain: &Chain,
    config: &AppConfig,
    fetcher: &F,
    all_specs: &std::collections::HashMap<String, QrPath>,
    all_metadata: &std::collections::HashMap<
        String,
        std::collections::BTreeMap<MetaVersion, QrPath>,
    >,
    online: Option<&ExportData>,
) -> Result<(String, ExportChainSpec)>
where
    F: Fetcher + Sync,
{
    let specs = match fetcher.fetch_specs(chain) {
        Ok(specs) => specs,
        Err(e) => {
            if let Some(online_specs) = online {
                if let Some(online_chain_specs) = online_specs.get(&chain.portal_id()) {
                    warn!(
                        "Unable to fetch specs for {}. Keep current online specs. Err: {}.",
                        chain.name, e
                    );
                    return Ok((chain.portal_id(), online_chain_specs.clone()));
                }
            }
            return Err(e);
        }
    };
    let meta = fetcher.fetch_metadata(chain)?;
    let live_meta_version = meta.meta_values.version;

    let metadata_qrs = collect_metadata_qrs(all_metadata, &chain.portal_id(), &live_meta_version)?;

    let specs_qr = all_specs
        .get(&chain.portal_id())
        .with_context(|| format!("No specs qr found for {}", chain.portal_id()))?
        .clone();
    let pointer_to_latest_meta = update_pointer_to_latest_metadata(
        metadata_qrs
            .first()
            .context(format!("No metadata QRs for {}", &chain.name))?,
    )?;
    Ok((
        chain.portal_id(),
        ExportChainSpec {
            title: chain.title.as_ref().unwrap_or(&chain.name).clone(),
            color: chain.color.clone(),
            rpc_endpoint: chain.rpc_endpoints[0].clone(), // keep only the first one
            genesis_hash: format!("0x{}", hex::encode(specs.genesis_hash)),
            unit: specs.unit,
            icon: chain.icon.clone(),
            decimals: specs.decimals,
            base58prefix: specs.base58prefix,
            specs_qr: QrCode::from_qr_path(config, specs_qr, &chain.verifier)?,
            latest_metadata: ReactAssetPath::from_fs_path(
                &pointer_to_latest_meta,
                &config.public_dir,
            )?,
            metadata_qr: export_live_metadata(
                config,
                metadata_qrs,
                &live_meta_version,
                &chain.verifier,
            ),
            live_meta_version,
            testnet: chain.testnet.unwrap_or(false),
            relay_chain: chain.relay_chain.clone(),
        },
    ))
}

fn export_live_metadata(
    config: &AppConfig,
    qrs: Vec<QrPath>,
    live_version: &MetaVersion,
    verifier_name: &String,
) -> Option<MetadataQr> {
    qrs.into_iter()
        .find(
            |qr| matches!(qr.file_name.content_type, ContentType::Metadata(v) if v==*live_version),
        )
        .map(|qr| MetadataQr {
            version: *live_version,
            file: QrCode::from_qr_path(config, qr, verifier_name).unwrap(),
        })
}

// Create symlink to latest metadata qr
fn update_pointer_to_latest_metadata(metadata_qr: &QrPath) -> Result<PathBuf> {
    let latest_metadata_qr = metadata_qr.dir.join(format!(
        "{}_metadata_latest.apng",
        metadata_qr.file_name.chain
    ));
    if latest_metadata_qr.is_symlink() {
        fs::remove_file(&latest_metadata_qr).unwrap();
    }
    symlink(metadata_qr.to_path_buf(), &latest_metadata_qr).unwrap();
    Ok(latest_metadata_qr)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::{env, fs};

    use definitions::crypto::Encryption;
    use definitions::metadata::MetaValues;
    use definitions::network_specs::NetworkSpecs;
    use generate_message::helpers::MetaFetched;
    use sp_core::H256;

    use super::*;
    use crate::config::Chain;

    struct MockFetcher;
    impl Fetcher for MockFetcher {
        fn fetch_specs(&self, _chain: &Chain) -> Result<NetworkSpecs> {
            Ok(NetworkSpecs {
                base58prefix: 0,
                color: "".to_string(),
                decimals: 10,
                encryption: Encryption::Ed25519,
                genesis_hash: H256::from_str(
                    "a8dfb73a4b44e6bf84affe258954c12db1fe8e8cf00b965df2af2f49c1ec11cd",
                )
                .expect("checked value"),
                logo: "logo".to_string(),
                name: "polkadot".to_string(),
                path_id: "".to_string(),
                secondary_color: "".to_string(),
                title: "".to_string(),
                unit: "DOT".to_string(),
            })
        }

        fn fetch_metadata(&self, _chain: &Chain) -> Result<MetaFetched> {
            Ok(MetaFetched {
                meta_values: MetaValues {
                    name: "".to_string(),
                    version: 9,
                    optional_base58prefix: None,
                    warn_incomplete_extensions: false,
                    meta: vec![],
                },
                block_hash: H256::zero(),
                genesis_hash: H256::zero(),
            })
        }
    }

    #[test]
    fn test_collector() {
        let root_dir = env::current_dir().unwrap();
        let config = AppConfig {
            qr_dir: root_dir.join("src/collector/for_tests"),
            public_dir: root_dir.join("src/collector"),
            ..Default::default()
        };

        let specs = export_specs(&config, MockFetcher).unwrap();
        let result = serde_json::to_string_pretty(&specs).unwrap();
        let expected = fs::read_to_string(config.qr_dir.join("expected.json"))
            .expect("unable to read expected file");
        assert_eq!(result, expected);

        let latest_symlink = config.qr_dir.join("polkadot_metadata_latest.apng");
        assert!(latest_symlink.exists());
        fs::remove_file(latest_symlink).unwrap();
    }
}
