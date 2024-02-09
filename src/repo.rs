use std::time::Duration;

use debcontrol::Paragraph;
use eyre::{eyre, Result};
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
    "loongarch64",
    "loongson3",
    "mips64r6el",
    "riscv64",
    "all",
];

#[derive(Debug, PartialEq, Clone)]
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
    let mut repo_pkgs = Vec::new();
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
                let entries = debcontrol::parse_str(&res).map_err(|e| eyre!("{e}"))?;
                repo_pkgs.extend(parse_packages_file(entries)?);
            }
            Err(e) => return Err(e),
        }
    }

    // sort by (name, arch) to find latest version for each (name, arch) pair
    repo_pkgs.sort_unstable_by(|a, b| a.name.cmp(&b.name).then(a.arch.cmp(&b.arch)));

    let mut versions = vec![];
    let mut last_entry = repo_pkgs
        .first()
        .ok_or_else(|| eyre!("Packages file is empty"))?;
    let mut result = vec![];

    for entry in &repo_pkgs {
        if entry.name == last_entry.name && entry.arch == last_entry.arch {
            versions.push((entry, PkgVersion::try_from(entry.version.as_str())?));
        } else {
            versions.sort_unstable_by(|a, b| a.1.cmp(&b.1));
            result.push(
                versions
                    .last()
                    .ok_or_else(|| eyre!("Packages file is empty"))?
                    .0
                    .to_owned()
                    .clone(),
            );
            versions.clear();
            last_entry = entry;
            versions.push((entry, PkgVersion::try_from(entry.version.as_str())?));
        }
    }

    Ok(result)
}

impl TryFrom<Paragraph<'_>> for RepoPackage {
    type Error = eyre::Error;

    fn try_from(value: Paragraph) -> std::prelude::v1::Result<Self, Self::Error> {
        let name = debcontrol_field(&value, "Package")?;
        let version = debcontrol_field(&value, "Version")?;
        let arch = debcontrol_field(&value, "Architecture")?;

        Ok(RepoPackage {
            name: name.to_string(),
            version: version.to_string(),
            arch: arch.to_string(),
        })
    }
}

fn debcontrol_field<'a>(value: &'a Paragraph<'a>, field: &str) -> Result<&'a String> {
    let field = &value
        .fields
        .iter()
        .find(|x| x.name == field)
        .ok_or_else(|| eyre!("Failed to get {field}"))?
        .value;

    Ok(field)
}

fn parse_packages_file(entries: Vec<Paragraph>) -> Result<Vec<RepoPackage>> {
    let mut new_entries = vec![];

    for i in entries
        .into_iter()
        .map(|x: Paragraph| RepoPackage::try_from(x))
    {
        let i = i?;
        new_entries.push(i);
    }

    Ok(new_entries)
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
