use anyhow::{anyhow, Result};
use std::{collections::HashMap, io::Read, path::Path};
use walkdir::WalkDir;

pub fn get_tree_package_list(tree: &Path) -> Result<HashMap<String, String>> {
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
