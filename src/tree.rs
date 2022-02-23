use anyhow::{anyhow, Result};
use log::warn;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use walkdir::WalkDir;

pub struct TreePackage {
    pub name: String,
    pub version: String,
    pub is_noarch: bool,
}

macro_rules! handle_parse {
    ($name:ident, $result:ident, $spec:ident, $defines:ident) => {
        let mut is_noarch = false;
        let mut ver = String::new();
        if let Some(v) = $spec.get("VER") {
            ver.push_str(v);
        }
        if let Some(rel) = $spec.get("REL") {
            ver = format!("{}-{}", ver, rel);
        }
        if let Some(epoch) = $defines.get("PKGEPOCH") {
            ver = format!("{}:{}", epoch, ver);
        }
        if $defines.get("ABHOST") == Some(&"noarch".to_string()) {
            is_noarch = true;
        }
        $result.push(TreePackage {
            name: $name,
            version: ver,
            is_noarch,
        });
    };
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
            let spec_parse = read_ab_with_apml(&mut spec).unwrap_or(
                {
                    warn!("Package {} Cannot use apml to parse spec file! fallback to read_ab_fallback function!", name);

                    read_ab_fallback(&mut spec)
                });
            let defines_parse =
                read_ab_with_apml(&mut defines).unwrap_or({
                    warn!("Package {} Cannot use apml to parse defines file! fallback to read_ab_fallback function!", name);

                    read_ab_fallback(&mut defines)
                });
            handle_parse!(name, result, spec_parse, defines_parse);
        }
    }

    result
}

fn read_ab_with_apml(file: &mut File) -> Result<HashMap<String, String>> {
    let mut file_buf = String::new();
    file.read_to_string(&mut file_buf)?;
    let mut context = HashMap::new();
    abbs_meta_apml::parse(&file_buf, &mut context)
        .map_err(|e| anyhow!(e.pretty_print(&file_buf, "File")))?;

    Ok(context)
}

fn read_ab_fallback(file: &mut File) -> HashMap<String, String> {
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut file_buf = String::new();
    file.read_to_string(&mut file_buf).unwrap();
    let mut context = HashMap::new();
    let split_file = file_buf.split('\n').collect::<Vec<_>>();
    handle_context(&split_file, &mut context, "VER");
    handle_context(&split_file, &mut context, "REL");
    handle_context(&split_file, &mut context, "PKGEPOCH");

    context
}

fn handle_context(split_file: &Vec<&str>, context: &mut HashMap<String, String>, key: &str) {
    let key_inner = &format!("{}=", key);
    let key_index = split_file.iter().position(|x| x.starts_with(key_inner));
    if let Some(index) = key_index {
        let value = split_file[index]
            .strip_prefix(key_inner)
            .unwrap()
            .replace("\"", "");
        context.insert(key.to_string(), value.to_string());
    }
}
