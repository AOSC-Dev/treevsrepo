use anyhow::Result;

macro_rules! BINARY_LIST_URL_RETRO {
    () => {
        "https://mirrors.bfsu.edu.cn/anthon/debs-retro/dists/stable/main/binary-{}/Packages"
    };
}

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

pub fn get_repo_package_ver_list(
    arch_list: Option<Vec<String>>,
) -> Result<Vec<(String, String, String)>> {
    let mut result = Vec::new();
    let mut all = Vec::new();
    let arch_list = if let Some(arch_list) = arch_list {
        arch_list
    } else {
        ARCH_LIST_RETRO
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
    };
    for i in &arch_list {
        let entry = get_list_from_repo(&i)?
            .split("\n")
            .map(|x| x.into())
            .collect::<Vec<String>>();
        all.extend(entry);
    }
    let mut last_index = 0;
    for (index, entry) in all.iter().enumerate() {
        if entry == &"" && index != last_index + 1 {
            let package_vec = &all[last_index..index];
            let package_name = get_value(package_vec, "Package");
            let version = get_value(package_vec, "Version");
            let arch = get_value(package_vec, "Architecture");
            result.push((
                package_name.to_string(),
                version.to_string(),
                arch.to_string(),
            ));
            last_index = index;
        }
    }

    Ok(result)
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

fn get_list_from_repo(binary_name: &str) -> Result<String> {
    let result = reqwest::blocking::get(&format!(BINARY_LIST_URL_RETRO!(), binary_name))?.text()?;

    Ok(result)
}