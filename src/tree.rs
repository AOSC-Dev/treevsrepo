use eyre::{anyhow, Context, Result};
use fancy_regex::Regex;
use log::{info, trace, warn};
use std::{
    collections::HashMap,
    fs::{self},
    path::Path,
};
use walkdir::WalkDir;

pub struct TreePackage {
    pub name: String,
    pub version: String,
    pub is_noarch: bool,
    pub fail_arch: Option<Regex>,
}

pub fn get_tree_package_list(tree: &Path) -> Result<Vec<TreePackage>> {
    let mut result = Vec::new();
    std::env::set_current_dir(tree).context("Cannot switch to tree directory")?;

    for entry in WalkDir::new(".")
        .max_depth(2)
        .min_depth(2)
        .into_iter()
        .flatten()
    {
        let name = entry.file_name().to_str().unwrap();
        if entry.file_type().is_dir() {
            let path = entry.path();
            let spec = path.join("spec");
            let spec = if path.join("spec").is_file() {
                spec
            } else {
                warn!("Package {} spec does not exist!", name);
                continue;
            };

            let defines_path = path.join("autobuild/defines");
            let defines_vec = if defines_path.is_file() {
                vec![defines_path]
            } else {
                // Try to walkdir group-package. like: 01-virtualbox
                info!(
                    "Package {} is group package? trying to search group package ...",
                    name
                );
                let mut result = Vec::new();
                for i in WalkDir::new(path)
                    .min_depth(2)
                    .max_depth(3)
                    .into_iter()
                    .flatten()
                {
                    if i.file_name().to_str() == Some("defines") {
                        result.push(i.path().to_path_buf());
                    }
                }
                if result.is_empty() {
                    warn!("Package {} defines does not exist!", name);
                    continue;
                }

                result
            };

            let spec = fs::read_to_string(spec)?;
            let spec_parse = read_ab_with_apml(&spec).unwrap_or({
                trace!("Package {} Cannot use apml to parse spec file! fallback to read_ab_fallback function!", name);

                read_ab_fallback(&spec)
            });
            for defines in defines_vec {
                let defines_file = fs::read_to_string(defines)?;
                let defines_parse = read_ab_with_apml(&defines_file).unwrap_or({
                    trace!("Package {} Cannot use apml to parse defines file! fallback to read_ab_fallback function!", name);

                    read_ab_fallback(&defines_file)
                });
                let mut is_noarch = false;
                let mut ver = String::new();
                let name = if let Some(pkgname) = defines_parse.get("PKGNAME") {
                    pkgname
                } else {
                    info!(
                        "Package {} defines has no PKGNAME! fallback to directory name ...",
                        name
                    );

                    name
                };
                if let Some(v) = spec_parse.get("VER") {
                    ver.push_str(v);
                } else {
                    warn!("Package {} has no version!", name);
                    continue;
                }
                if let Some(rel) = spec_parse.get("REL") {
                    ver = format!("{}-{}", ver, rel);
                }
                if let Some(epoch) = defines_parse.get("PKGEPOCH") {
                    ver = format!("{}:{}", epoch, ver);
                }

                let fail_arch = if let Some(fail_arch) = defines_parse.get("FAIL_ARCH") {
                    fail_arch_regex(fail_arch).ok()
                } else {
                    None
                };

                if defines_parse.get("ABHOST") == Some(&"noarch".to_string()) {
                    is_noarch = true;
                }
                result.push(TreePackage {
                    name: name.to_string(),
                    version: ver.to_string(),
                    is_noarch,
                    fail_arch,
                });
            }
        }
    }

    Ok(result)
}

fn read_ab_with_apml(file: &str) -> Result<HashMap<String, String>> {
    let mut context = HashMap::new();

    // Try to set some ab3 flags to reduce the chance of returning errors
    for i in ["ARCH", "PKGDIR", "SRCDIR"] {
        context.insert(i.to_string(), "".to_string());
    }

    abbs_meta_apml::parse(&file, &mut context).map_err(|e| {
        let e: Vec<String> = e.iter().map(|e| e.to_string()).collect();
        anyhow!(e.join(": "))
    })?;

    Ok(context)
}

fn read_ab_fallback(file: &str) -> HashMap<String, String> {
    let mut context = HashMap::new();
    let split_file = file.split('\n').collect::<Vec<_>>();

    for i in ["VER", "REL", "PKGNAME", "PKGEPOCH", "FAIL_ARCH", "ABHOST"] {
        handle_context(&split_file, &mut context, i);
    }

    context
}

fn handle_context(split_file: &[&str], context: &mut HashMap<String, String>, key: &str) {
    let key_inner = &format!("{}=", key);
    let key_index = split_file.iter().position(|x| x.starts_with(key_inner));
    if let Some(index) = key_index {
        let value = split_file[index]
            .strip_prefix(key_inner)
            .unwrap()
            .replace('\"', "");
        context.insert(key.to_string(), value);
    }
}

fn fail_arch_regex(expr: &str) -> Result<Regex> {
    let mut regex = String::from("^");
    let mut negated = false;
    let mut sup_bracket = false;
    if expr.len() < 3 {
        return Err(anyhow!("Pattern too short."));
    }
    let expr = expr.as_bytes();
    for (i, c) in expr.iter().enumerate() {
        if i == 0 && c == &b'!' {
            negated = true;
            if expr.get(1) != Some(&b'(') {
                regex += "(";
                sup_bracket = true;
            }
            continue;
        }
        if negated {
            if c == &b'(' {
                regex += "(?!";
                continue;
            } else if i == 1 && sup_bracket {
                regex += "?!";
            }
        }
        regex += std::str::from_utf8(&[*c])?;
    }
    if sup_bracket {
        regex += ")";
    }

    Ok(Regex::new(&regex)?)
}

#[test]
fn test_fail_arch_regex() {
    let fail_arch = "!(amd64|arm64)";
    let reg = fail_arch_regex(fail_arch).unwrap();

    assert!(!reg.is_match("amd64").unwrap());
    assert!(!reg.is_match("arm64").unwrap());
    assert!(reg.is_match("ppc64el").unwrap());
}
