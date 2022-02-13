use std::collections::HashMap;

use anyhow::Result;

const BASE_URL: &str = "https://repo.aosc.io/debs-retro/";

pub fn get_repo_package_ver_list() -> Result<HashMap<String, (String, String)>> {
    let mut result = HashMap::new();
    let binary_i486 = get_list_from_repo("dists/stable/main/binary-i486/Packages")?;
    let binary_all = get_list_from_repo("dists/stable/main/binary-all/Packages")?;
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
            let package_name = get_value(package_vec, "Package");
            let version = get_value(package_vec, "Version");
            let arch = get_value(package_vec, "Architecture");
            result.insert(
                package_name.to_string(),
                (version.to_string(), arch.to_string()),
            );
            last_index = index;
        }
    }

    Ok(result)
}

fn get_value(package_vec: &[&str], value: &str) -> String {
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

fn get_list_from_repo(url: &str) -> Result<String> {
    let result = reqwest::blocking::get(format!("{}/{}", BASE_URL, url))?.text()?;

    Ok(result)
}
