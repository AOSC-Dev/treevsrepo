use anyhow::{anyhow, Result};
use clap::Parser;
use std::{collections::HashMap, io::Read, path::Path};
use walkdir::WalkDir;

const BASE_URL: &str = "https://repo.aosc.io/debs-retro/";

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long)]
    tree: String,
    #[clap(short = 'o', long)]
    output: Option<String>,
}

struct TreeVsRepo {
    name: String,
    arch: String,
    tree_version: String,
    repo_version: String,
}

fn main() {
    let args = Args::parse();
    let now_env = std::env::current_dir().expect("Cannot get your env!");
    let repo_map = get_repo_package_ver_list().unwrap();
    let tree_map = get_tree_package_list(Path::new(&args.tree)).unwrap();
    let result = get_result(repo_map, tree_map);
    if let Some(output) = args.output {
        let mut file_vec = Vec::new();
        for i in result {
            file_vec.push(i.name);
        }
        file_vec.sort();
        let file_str = file_vec.join("\n");
        std::fs::write(now_env.join(output), file_str).unwrap();
    } else {
        println!(
            "{:<30}{:<30}{:<30}\t\tArch",
            "Name", "Tree version", "Repo version"
        );
        for i in result {
            println!(
                "{:<30}{:<30}{:<30}\t\t{}",
                i.name, i.tree_version, i.repo_version, i.arch
            );
        }
    }
}

fn get_tree_package_list(tree: &Path) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    std::env::set_current_dir(tree)
        .map_err(|e| anyhow!("Cannot switch to tree directory! why: {}", e))?;
    for entry in WalkDir::new(".")
        .max_depth(2)
        .min_depth(2)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_str().unwrap();
            let path = entry.path();
            let spec = std::fs::File::open(path.join("spec"));
            let mut defines =
                if let Ok(defines) = std::fs::File::open(path.join("autobuild/defines")) {
                    defines
                } else {
                    continue;
                };
            if let Ok(mut spec) = spec {
                let mut spec_buf = String::new();
                spec.read_to_string(&mut spec_buf)?;
                let spec_vec = spec_buf.split('\n').collect::<Vec<_>>();
                let mut defines_buf = String::new();
                defines.read_to_string(&mut defines_buf)?;
                let defines_vec = defines_buf.split('\n').collect::<Vec<_>>();
                let ver_index = spec_vec.iter().position(|x| x.contains("VER=")).unwrap();
                let ver = spec_vec[ver_index].strip_prefix("VER=");
                let rel = spec_vec.iter().position(|x| x.contains("REL="));
                let epoch_index = defines_vec.iter().position(|x| x.contains("PKGEPOCH="));
                if let Some(ver) = ver {
                    let mut ver = ver.to_string();
                    if let Some(rel) = rel {
                        ver = format!("{}-{}", ver, rel);
                    }
                    if let Some(epoch_index) = epoch_index {
                        let epoch = defines_vec[epoch_index].strip_prefix("PKGEPOCH=").unwrap();
                        ver = format!("{}:{}", epoch, ver);
                    }
                    result.insert(name.to_string(), ver);
                }
            }
        }
    }

    Ok(result)
}

fn get_repo_package_ver_list() -> Result<HashMap<String, (String, String)>> {
    let mut result = HashMap::new();
    let binary_i486 = reqwest::blocking::get(format!(
        "{}/{}",
        BASE_URL, "dists/stable/main/binary-i486/Packages"
    ))?
    .text()?;
    let binary_all = reqwest::blocking::get(format!(
        "{}/{}",
        BASE_URL, "dists/stable/main/binary-all/Packages"
    ))?
    .text()?;
    let binary_i486_vec = binary_i486.split('\n');
    let binary_all_vec = binary_all.split('\n');
    let mut last_index = 0;
    let all = binary_i486_vec
        .into_iter()
        .chain(binary_all_vec)
        .collect::<Vec<_>>();
    for (index, entry) in all.iter().enumerate() {
        if entry == &"" && index != last_index + 1 {
            let package_vec = &all[last_index..index];
            let package_name_index = package_vec
                .iter()
                .position(|x| x.contains("Package: "))
                .unwrap();
            let package_name = package_vec[package_name_index]
                .strip_prefix("Package: ")
                .unwrap();
            let version_index = package_vec
                .iter()
                .position(|x| x.contains("Version: "))
                .unwrap();
            let arch_index = package_vec
                .iter()
                .position(|x| x.contains("Architecture: "))
                .unwrap();
            let version = package_vec[version_index]
                .strip_prefix("Version: ")
                .unwrap();
            let arch = package_vec[arch_index]
                .strip_prefix("Architecture: ")
                .unwrap();
            result.insert(
                package_name.to_string(),
                (version.to_string(), arch.to_string()),
            );
            last_index = index;
        }
    }

    Ok(result)
}

fn get_result(
    repo_map: HashMap<String, (String, String)>,
    tree_map: HashMap<String, String>,
) -> Vec<TreeVsRepo> {
    let mut result = Vec::new();
    for (k, v) in tree_map {
        if let Some((repo_version, arch)) = repo_map.get(&k) {
            if &v != repo_version {
                result.push(TreeVsRepo {
                    name: k,
                    arch: arch.to_string(),
                    tree_version: v,
                    repo_version: repo_version.to_string(),
                });
            };
        }
    }

    result
}
