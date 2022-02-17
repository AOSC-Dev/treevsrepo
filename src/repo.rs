use anyhow::Result;

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

#[derive(Debug, PartialEq)]
pub struct RepoPackage {
    pub name: String,
    pub version: String,
    pub arch: String,
}

pub fn get_repo_package_ver_list(
    mirror: &str,
    arch_list: Option<Vec<String>>,
) -> Result<Vec<RepoPackage>> {
    let mut result = Vec::new();
    let arch_list = if let Some(arch_list) = arch_list {
        arch_list
    } else {
        ARCH_LIST_RETRO
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
    };
    for i in &arch_list {
        let entrys = get_list_from_repo(i, mirror)?
            .split('\n')
            .map(|x| x.into())
            .collect::<Vec<String>>();
        result.extend(handle(entrys));
    }

    Ok(result)
}

fn handle(entrys: Vec<String>) -> Vec<RepoPackage> {
    let mut last_index = 0;
    let mut result = Vec::new();
    let mut temp_vec = Vec::new();
    let mut last_name = entrys
        .first()
        .unwrap()
        .strip_prefix("Package: ")
        .unwrap()
        .to_string();
    for (index, entry) in entrys.iter().enumerate() {
        if entry.is_empty() && index != entrys.len() - 1 {
            let package_vec = &entrys[last_index..index];
            let package_name = get_value(package_vec, "Package");
            let version = get_value(package_vec, "Version");
            let arch = get_value(package_vec, "Architecture");
            if last_name == package_name {
                temp_vec.push((
                    package_name.to_string(),
                    version.to_string(),
                    arch.to_string(),
                ));
            } else {
                let (name, version, arch) = temp_vec.last().unwrap().to_owned();
                let repo_package = RepoPackage {
                    name,
                    version: version.to_string(),
                    arch: arch.to_string(),
                };
                result.push(repo_package);
                temp_vec.clear();
                temp_vec.push((
                    package_name.to_string(),
                    version.to_string(),
                    arch.to_string(),
                ));
                last_name = package_name;
            }
            last_index = index;
        } else if index == entrys.len() - 1 {
            let (name, version, arch) = temp_vec.last().unwrap().to_owned();
            let repo_package = RepoPackage {
                name,
                version,
                arch,
            };
            result.push(repo_package);
        }
    }

    result
}

fn get_value(package_vec: &[String], value: &str) -> String {
    let index = package_vec
        .iter()
        .position(|x| x.contains(&format!("{}: ", value)))
        .unwrap();
    let result = package_vec[index]
        .strip_prefix(&format!("{}: ", value))
        .unwrap()
        .to_string();

    result
}

fn get_list_from_repo(binary_name: &str, mirror: &str) -> Result<String> {
    let url = if mirror.ends_with('/') {
        mirror.to_string()
    } else {
        format!("{}/", mirror)
    };
    let url = format!(
        "{}debs-retro/dists/stable/main/binary-{}/Packages",
        url, binary_name
    );
    let result = reqwest::blocking::get(url)?.error_for_status()?.text()?;

    Ok(result)
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
