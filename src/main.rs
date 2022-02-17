use clap::Parser;
use repo::RepoPackage;
use std::path::Path;
use tree::TreePackage;

mod repo;
mod tree;

const DEFAULT_URL: &str = "https://repo.aosc.io";

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// Set tree directory. e.g: /home/saki/aosc-os-abbs
    #[clap(short, long)]
    tree: String,
    /// Output result to file.
    #[clap(short = 'o', long, requires = "arch")]
    output: Option<String>,
    /// Set search arch.
    #[clap(short, long, min_values = 1)]
    arch: Option<Vec<String>>,
    /// Set mirror.
    #[clap(short = 'm', long, default_value = DEFAULT_URL)]
    mirror: String,
}

struct TreeVsRepo {
    name: String,
    arch: String,
    tree_version: String,
    repo_version: String,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let now_env = std::env::current_dir().expect("Cannot get your env!");
    let arch = args.arch;
    if let Some(output) = args.output {
        let repo_map = repo::get_repo_package_ver_list(&args.mirror, arch).unwrap();
        let tree_map = tree::get_tree_package_list(Path::new(&args.tree));
        let result = get_result(repo_map, tree_map);
        result_to_file(result, output, now_env);
    } else {
        let repo_map = repo::get_repo_package_ver_list(&args.mirror, arch).unwrap();
        let tree_map = tree::get_tree_package_list(Path::new(&args.tree));
        let result = get_result(repo_map, tree_map);
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

fn result_to_file(result: Vec<TreeVsRepo>, output: String, now_env: std::path::PathBuf) {
    let mut file_vec = Vec::new();
    for i in result {
        if !file_vec.contains(&i.name) {
            file_vec.push(i.name);
        }
    }
    let file_str = file_vec.join("\n");
    let path = Path::new(&output);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        now_env.join(output)
    };
    std::fs::write(path, file_str).unwrap();
}

fn get_result(repo_vec: Vec<RepoPackage>, tree_vec: Vec<TreePackage>) -> Vec<TreeVsRepo> {
    let mut result = Vec::new();
    for tree_package in tree_vec {
        let filter_vec = repo_vec
            .iter()
            .filter(|x| x.name == tree_package.name)
            .collect::<Vec<_>>();
        for repo_package in filter_vec {
            if tree_package.version != repo_package.version {
                result.push(TreeVsRepo {
                    name: tree_package.name.to_string(),
                    arch: repo_package.arch.to_string(),
                    tree_version: tree_package.version.to_string(),
                    repo_version: repo_package.version.to_string(),
                });
            }
        }
    }

    result
}
