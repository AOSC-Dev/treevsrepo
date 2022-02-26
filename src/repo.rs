use std::time::Duration;

use anyhow::Result;
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

const ARCH_LIST_MAINLINE: &[&str] = &["amd64", "arm64", "ppc64el", "loongson3", "riscv64", "all"];

#[derive(Debug, PartialEq)]
pub struct RepoPackage {
    pub name: String,
    pub version: String,
    pub arch: String,
}

pub fn get_repo_package_ver_list(
    mirror: &str,
    arch_list: Option<Vec<String>>,
    is_retro: bool,
) -> Result<Vec<RepoPackage>> {
    let mut result = Vec::new();
    let arch_list = if let Some(arch_list) = arch_list {
        arch_list
    } else if is_retro {
        ARCH_LIST_RETRO
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
    } else {
        ARCH_LIST_MAINLINE
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
    };
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    let client = reqwest::Client::new();
    runtime.block_on(async move {
        let mut task = Vec::new();
        for i in &arch_list {
            task.push(get_list_from_repo(i, mirror, &client));
        }
        let results = futures::future::join_all(task).await;
        for i in results {
            match i {
                Ok(res) => {
                    let entrys = res.split('\n').map(|x| x.into()).collect::<Vec<String>>();
                    result.extend(handle(entrys));
                }
                Err(e) => return Err(e),
            }
        }

        Ok(result)
    })
}

fn handle(entrys: Vec<String>) -> Vec<RepoPackage> {
    let mut result = Vec::new();
    let entrys = parse_package_file(entrys);
    let mut pushed_package = Vec::new();
    for entry in &entrys {
        if pushed_package.contains(&entry.name) {
            continue;
        }
        let filter_vec = entrys
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

fn get_value<'a>(package_vec: &'a [String], value: &'a str) -> &'a str {
    let index = package_vec
        .iter()
        .position(|x| x.contains(&format!("{}: ", value)))
        .unwrap();
    let result = package_vec[index]
        .strip_prefix(&format!("{}: ", value))
        .unwrap();

    result
}

async fn get_list_from_repo(binary_name: &str, mirror: &str, client: &Client) -> Result<String> {
    let url = if mirror.ends_with('/') {
        mirror.to_string()
    } else {
        format!("{}/", mirror)
    };
    let directory_name = if ARCH_LIST_MAINLINE.contains(&binary_name) {
        "debs"
    } else {
        "debs_retro"
    };
    let url = format!(
        "{}{}/dists/stable/main/binary-{}/Packages",
        url, directory_name, binary_name
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

fn parse_package_file(entrys: Vec<String>) -> Vec<RepoPackage> {
    let mut last_index = 0;
    let mut result = Vec::new();
    for (index, entry) in entrys.iter().enumerate() {
        if entry.is_empty() && index != entrys.len() - 1 {
            let package_vec = &entrys[last_index..index];
            let package_name = get_value(package_vec, "Package");
            let version = get_value(package_vec, "Version");
            let arch = get_value(package_vec, "Architecture");
            let repo_package = RepoPackage {
                name: package_name.to_string(),
                version: version.to_string(),
                arch: arch.to_string(),
            };
            result.push(repo_package);
            last_index = index;
        }
    }

    result
}

#[test]
fn test_handle() {
    let s = "Package: qaq\nVersion: 1.0\nArchitecture: qwq\n\nPackage: qaq\nVersion: 1.1\nArchitecture: qwq\n\nPackage: aaaa\nVersion: 2.0\nArchitecture: qwq\n\n";
    let entrys = s
        .split('\n')
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<String>>();

    assert_eq!(
        handle(entrys),
        vec![
            RepoPackage {
                name: "qaq".to_string(),
                version: "1.1".to_string(),
                arch: "qwq".to_string(),
            },
            RepoPackage {
                name: "aaaa".to_string(),
                version: "2.0".to_string(),
                arch: "qwq".to_string()
            }
        ]
    );
}
