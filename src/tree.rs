use anyhow::{anyhow, Result};
use log::warn;
use std::{collections::HashMap, fs::File, io::Read, path::Path};
use walkdir::WalkDir;

pub struct TreePackage {
    pub name: String,
    pub version: String,
    pub is_noarch: bool,
}

pub fn get_tree_package_list(tree: &Path) -> Vec<TreePackage> {
    let mut result = Vec::new();
    std::env::set_current_dir(tree)
        .map_err(|e| anyhow!("Cannot switch to tree directory! why: {}", e))
        .unwrap();
    for entry in WalkDir::new(".")
        .max_depth(2)
        .min_depth(2)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_str().unwrap().to_owned();
            let path = entry.path();
            let mut spec = if let Ok(spec) = std::fs::File::open(path.join("spec")) {
                spec
            } else {
                warn!("Package {} spec does not exist!", name);
                continue;
            };
            let mut defines =
                if let Ok(defines) = std::fs::File::open(path.join("autobuild/defines")) {
                    defines
                } else {
                    warn!("Package {} defines does not exist!", name);
                    continue;
                };
            let mut is_noarch = false;
            let spec = read_ab(&mut spec);
            let defines = read_ab(&mut defines);
            if let Ok(spec) = spec {
                let mut ver = String::new();
                if let Some(v) = spec.get("VER") {
                    ver.push_str(v);
                }
                if let Some(rel) = spec.get("REL") {
                    ver = format!("{}-{}", ver, rel);
                }
                if let Ok(defines) = defines {
                    if let Some(epoch) = defines.get("PKGEPOCH") {
                        ver = format!("{}:{}", epoch, ver);
                    }
                    if defines.get("ABHOST") == Some(&"noarch".to_string()) {
                        is_noarch = true;
                    }
                } else {
                    defines.unwrap_err();
                }
                result.push((name, ver, is_noarch));
            } else {
                spec.unwrap_err();
            };
        }
    }
    result.sort();
    let result = result
        .into_iter()
        .map(|(name, ver, is_noarch)| TreePackage {
            name,
            version: ver,
            is_noarch,
        })
        .collect::<Vec<_>>();

    result
}

fn read_ab(file: &mut File) -> Result<HashMap<String, String>> {
    let mut file_buf = String::new();
    file.read_to_string(&mut file_buf)?;
    let mut context = HashMap::new();
    abbs_meta_apml::parse(&file_buf, &mut context)?;

    Ok(context)
}
