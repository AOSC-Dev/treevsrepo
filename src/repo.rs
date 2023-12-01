use std::time::Duration;

use anyhow::{anyhow, Result};
use debcontrol::Paragraph;
use reqwest::Client;
use tokio::runtime::Builder;

use crate::pkgversion::PkgVersion;

const ARCH_LIST_RETRO: &[&str] = &[
    "i486",
    "armel",
    "armhf",
    "armv4",
    "loongson2f",
    "powerpc",
    "ppc64",
    "all",
];

const ARCH_LIST_MAINLINE: &[&str] = &[
    "amd64",
    "arm64",
    "ppc64el",
    "loongson3",
    "mips64r6el",
    "riscv64",
    "all",
];

#[derive(Debug, PartialEq)]
pub struct RepoPackage {
    pub name: String,
    pub version: String,
    pub arch: String,
}

pub fn get_repo_package_ver_list(
    mirror: &str,
    topic: &str,
    arch_list: Vec<String>,
    is_retro: bool,
) -> Result<Vec<RepoPackage>> {
    let mut result = Vec::new();
    let arch_list = if !arch_list.is_empty() {
        arch_list.iter().map(|x| x.as_str()).collect::<Vec<_>>()
    } else if is_retro {
        ARCH_LIST_RETRO.to_owned()
    } else {
        ARCH_LIST_MAINLINE.to_owned()
    };

    let runtime = Builder::new_multi_thread().enable_all().build()?;
    let client = reqwest::Client::new();

    let results = runtime.block_on(async move {
        let mut task = Vec::new();
        for i in &arch_list {
            task.push(get_list_from_repo(i, mirror, topic, &client));
        }

        futures::future::join_all(task).await
    });

    for i in results {
        match i {
            Ok(res) => {
                let entries = debcontrol::parse_str(&res).map_err(|e| anyhow!("{}", e))?;
                result.extend(handle(entries));
            }
            Err(e) => return Err(e),
        }
    }

    Ok(result)
}

fn handle(entries: Vec<Paragraph>) -> Vec<RepoPackage> {
    let mut result = Vec::new();
    let entries = parse_package_file(entries);
    let mut pushed_package = Vec::new();
    for entry in &entries {
        if pushed_package.contains(&entry.name) {
            continue;
        }
        let filter_vec = entries
            .iter()
            .filter(|x| x.name == entry.name)
            .collect::<Vec<_>>();
        let mut parse_vec = Vec::new();
        for i in filter_vec {
            parse_vec.push((
                i.name.to_string(),
                PkgVersion::try_from(i.version.as_str()).unwrap(),
                i.arch.to_string(),
            ));
        }
        parse_vec.sort_by(|x, y| x.1.cmp(&y.1));
        let (last_name, last_version, last_arch) = parse_vec.last().unwrap();
        result.push(RepoPackage {
            name: last_name.to_string(),
            version: last_version.to_string(),
            arch: last_arch.to_string(),
        });
        pushed_package.push(last_name.to_string());
    }

    result
}

async fn get_list_from_repo(
    binary_name: &str,
    mirror: &str,
    topic: &str,
    client: &Client,
) -> Result<String> {
    let url = if mirror.ends_with('/') {
        mirror.to_string()
    } else {
        format!("{}/", mirror)
    };
    let directory_name = if ARCH_LIST_MAINLINE.contains(&binary_name) {
        "debs"
    } else {
        "debs-retro"
    };
    let url = format!(
        "{}{}/dists/{}/main/binary-{}/Packages",
        url, directory_name, topic, binary_name
    );
    let result = client
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(result)
}

fn parse_package_file(entries: Vec<Paragraph>) -> Vec<RepoPackage> {
    let mut result = Vec::new();
    for entry in entries {
        let package_name = &entry
            .fields
            .iter()
            .find(|x| x.name == "Package")
            .unwrap()
            .value;
        let version = &entry
            .fields
            .iter()
            .find(|x| x.name == "Version")
            .unwrap()
            .value;
        let arch = &entry
            .fields
            .iter()
            .find(|x| x.name == "Architecture")
            .unwrap()
            .value;
        let repo_package = RepoPackage {
            name: package_name.to_string(),
            version: version.to_string(),
            arch: arch.to_string(),
        };
        result.push(repo_package);
    }

    result
}
