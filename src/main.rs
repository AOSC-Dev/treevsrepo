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

#[derive(Debug, PartialEq)]
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
    let mut all_no_match = Vec::new();
    for tree_package in tree_vec.iter() {
        let repo_filter_vec = repo_vec
            .iter()
            .filter(|x| x.name == tree_package.name)
            .collect::<Vec<_>>();
        for repo_package in repo_filter_vec.iter() {
            if tree_package.version != repo_package.version {
                if tree_package.is_noarch && repo_package.arch != "all" {
                    if repo_filter_vec
                        .iter()
                        .any(|x| x.arch == "all" && x.version == tree_package.version)
                    {
                        continue;
                    } else if repo_filter_vec
                        .iter()
                        .all(|x| x.version != tree_package.version)
                    {
                        if all_no_match
                            .iter()
                            .all(|x: &&RepoPackage| x.name != repo_package.name)
                            && !result
                                .iter().any(|x: &TreeVsRepo| x.name == repo_package.name)
                        {
                            all_no_match.push(&(*(*repo_package)));
                        }
                        continue;
                    }
                } else if !tree_package.is_noarch && repo_package.arch == "all" {
                    if repo_filter_vec
                        .iter()
                        .any(|x| x.arch != "all" && x.version == tree_package.version)
                    {
                        continue;
                    } else if repo_filter_vec
                        .iter()
                        .all(|x| x.version != tree_package.version)
                    {
                        if all_no_match
                            .iter()
                            .all(|x: &&RepoPackage| x.name != repo_package.name)
                            && !result
                                .iter().any(|x: &TreeVsRepo| x.name == repo_package.name)
                        {
                            all_no_match.push(repo_package);
                        }
                        continue;
                    }
                }
                if all_no_match.iter().all(|x| x.name != repo_package.name) {
                    result.push(TreeVsRepo {
                        name: repo_package.name.to_string(),
                        arch: repo_package.arch.to_string(),
                        tree_version: tree_package.version.to_string(),
                        repo_version: repo_package.version.to_string(),
                    });
                }
            }
        }
    }
    for i in all_no_match {
        let tree_index = tree_vec.iter().position(|x| x.name == i.name).unwrap();
        let tree_version = tree_vec[tree_index].version.to_string();
        let is_noarch = tree_vec[tree_index].is_noarch;
        if is_noarch {
            let repo_index = repo_vec
                .iter()
                .position(|x| x.name == i.name && x.arch == "all")
                .unwrap();
            let repo_version = repo_vec[repo_index].version.to_string();
            result.push(TreeVsRepo {
                name: i.name.to_string(),
                tree_version,
                arch: "all".to_string(),
                repo_version,
            })
        } else {
            let repo_all_match = repo_vec
                .iter()
                .filter(|x| x.name == i.name && x.arch != "all")
                .collect::<Vec<_>>();
            for j in repo_all_match {
                result.push(TreeVsRepo {
                    name: j.name.to_string(),
                    arch: j.arch.to_string(),
                    tree_version: tree_version.to_string(),
                    repo_version: j.version.to_string(),
                });
            }
        }
    }
    result.sort_by(|x, y| x.name.cmp(&y.name));

    result
}

#[test]
fn test_get_result_1() {
    let repo_vec = vec![RepoPackage {
        name: "qaq".to_string(),
        version: "114514".to_string(),
        arch: "all".to_string(),
    }];
    let tree_vec = vec![TreePackage {
        name: "qaq".to_string(),
        version: "114514".to_string(),
        is_noarch: true,
    }];

    assert!(get_result(repo_vec, tree_vec).is_empty());
}

#[test]
fn test_get_result_2() {
    let repo_vec = vec![
        RepoPackage {
            name: "qaq".to_string(),
            version: "114514".to_string(),
            arch: "owo".to_string(),
        },
        RepoPackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            arch: "all".to_string(),
        },
    ];
    let tree_vec = vec![TreePackage {
        name: "qaq".to_string(),
        version: "1:1.0".to_string(),
        is_noarch: true,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec),
        vec![TreeVsRepo {
            name: "qaq".to_string(),
            arch: "all".to_string(),
            tree_version: "1:1.0".to_string(),
            repo_version: "1.0".to_string(),
        }],
    )
}

#[test]
fn test_get_result_3() {
    let repo_vec = vec![
        RepoPackage {
            name: "qaq".to_string(),
            version: "114513".to_string(),
            arch: "owo".to_string(),
        },
        RepoPackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            arch: "all".to_string(),
        },
    ];
    let tree_vec = vec![TreePackage {
        name: "qaq".to_string(),
        version: "114514".to_string(),
        is_noarch: false,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec),
        vec![TreeVsRepo {
            name: "qaq".to_string(),
            arch: "owo".to_string(),
            tree_version: "114514".to_string(),
            repo_version: "114513".to_string(),
        }],
    )
}

#[test]
fn test_get_result_4() {
    let repo_vec = vec![
        RepoPackage {
            name: "qaq".to_string(),
            version: "114513".to_string(),
            arch: "owo".to_string(),
        },
        RepoPackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            arch: "pwp".to_string(),
        },
    ];
    let tree_vec = vec![TreePackage {
        name: "qaq".to_string(),
        version: "114514".to_string(),
        is_noarch: false,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec),
        vec![
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "owo".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "114513".to_string(),
            },
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "pwp".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "1.0".to_string(),
            }
        ],
    )
}

#[test]
fn test_get_result_5() {
    let repo_vec = vec![
        RepoPackage {
            name: "qaq".to_string(),
            version: "114513".to_string(),
            arch: "owo".to_string(),
        },
        RepoPackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            arch: "pwp".to_string(),
        },
        RepoPackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            arch: "all".to_string(),
        },
    ];
    let tree_vec = vec![TreePackage {
        name: "qaq".to_string(),
        version: "114514".to_string(),
        is_noarch: false,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec),
        vec![
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "owo".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "114513".to_string(),
            },
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "pwp".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "1.0".to_string(),
            }
        ],
    )
}

#[test]
fn test_get_result_6() {
    let repo_vec = vec![
        RepoPackage {
            name: "qaq".to_string(),
            version: "114513".to_string(),
            arch: "owo".to_string(),
        },
        RepoPackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            arch: "pwp".to_string(),
        },
        RepoPackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            arch: "all".to_string(),
        },
    ];
    let tree_vec = vec![TreePackage {
        name: "qaq".to_string(),
        version: "1.0".to_string(),
        is_noarch: true,
    }];

    assert!(get_result(repo_vec, tree_vec).is_empty())
}
