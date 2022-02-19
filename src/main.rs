use clap::Parser;
use std::path::Path;

mod repo;
mod tree;
mod vs;

const DEFAULT_URL: &str = "https://repo.aosc.io";

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// Set tree directory. e.g: /home/saki/aosc-os-abbs
    #[clap(short = 't', long)]
    tree: String,
    /// Output result to file.
    #[clap(short = 'o', long, requires = "arch")]
    output: Option<String>,
    /// Set search arch.
    #[clap(short = 'a', long, min_values = 1)]
    arch: Option<Vec<String>>,
    /// Set mirror.
    #[clap(short = 'm', long, default_value = DEFAULT_URL)]
    mirror: String,
}
fn main() {
    env_logger::init();
    let args = Args::parse();
    let now_env = std::env::current_dir().expect("Cannot get your env!");
    let arch = args.arch;
    if let Some(output) = args.output {
        let repo_map = repo::get_repo_package_ver_list(&args.mirror, arch).unwrap();
        let tree_map = tree::get_tree_package_list(Path::new(&args.tree));
        let result = vs::get_result(repo_map, tree_map);
        vs::result_to_file(result, output, now_env);
    } else {
        let repo_map = repo::get_repo_package_ver_list(&args.mirror, arch).unwrap();
        let tree_map = tree::get_tree_package_list(Path::new(&args.tree));
        let result = vs::get_result(repo_map, tree_map);
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