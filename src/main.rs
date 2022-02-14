use clap::Parser;
use std::path::Path;

mod repo;
mod tree;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long)]
    tree: String,
    #[clap(short = 'o', long)]
    output: Option<String>,
    #[clap(short, long, min_values = 1)]
    arch: Option<Vec<String>>,
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
    let repo_map = repo::get_repo_package_ver_list(arch).unwrap();
    let tree_map = tree::get_tree_package_list(Path::new(&args.tree)).unwrap();
    let result = get_result(repo_map, tree_map);
    if let Some(output) = args.output {
        let mut file_vec = Vec::new();
        for i in result {
            file_vec.push(i.name);
        }
        let file_str = file_vec.join("\n");
        let path = Path::new(&output);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            now_env.join(output)
        };
        std::fs::write(path, file_str).unwrap();
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

fn get_result(
    repo_vec: Vec<(String, String, String)>,
    tree_vec: Vec<(String, String)>,
) -> Vec<TreeVsRepo> {
    let mut result = Vec::new();
    for (package, tree_version) in tree_vec {
        let filter_map = repo_vec
            .iter()
            .filter(|(p, _, _)| p == &package)
            .collect::<Vec<_>>();
        for (_, repo_version, arch) in filter_map {
            if &tree_version != repo_version {
                result.push(TreeVsRepo {
                    name: package.to_string(),
                    arch: arch.to_string(),
                    tree_version: tree_version.to_string(),
                    repo_version: repo_version.to_string(),
                });
            };
        }
    }

    result
}
