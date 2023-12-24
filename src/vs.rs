use oma_apt::util::cmp_versions;
use tabled::Tabled;

use crate::repo::RepoPackage;
use crate::tree::TreePackage;
use eyre::{eyre, Result};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Tabled, Debug, PartialEq)]
pub struct TreeVsRepo {
    pub name: String,
    pub arch: String,
    pub tree_version: String,
    pub repo_version: String,
    #[tabled(skip)]
    pub compare: DpkgCompare,
}

#[derive(Tabled, Debug, PartialEq)]
pub enum DpkgCompare {
    Less,
    Equal,
    Greeter,
}

pub fn get_result(
    repo_vec: Vec<RepoPackage>,
    tree_vec: Vec<TreePackage>,
) -> Result<Vec<TreeVsRepo>> {
    let mut result = Vec::new();
    let mut all_no_match = Vec::new();
    for tree_package in tree_vec.iter() {
        let repo_filter_vec = repo_vec
            .iter()
            .filter(|x| x.name == tree_package.name)
            .collect::<Vec<_>>();
        for repo_package in repo_filter_vec.iter() {
            if tree_package.version == repo_package.version {
                continue;
            }
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
                            .iter()
                            .any(|x: &TreeVsRepo| x.name == repo_package.name)
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
                            .iter()
                            .any(|x: &TreeVsRepo| x.name == repo_package.name)
                    {
                        all_no_match.push(repo_package);
                    }
                    continue;
                }
            }
            if all_no_match.iter().all(|x| x.name != repo_package.name) {
                let mut push = true;

                if tree_package
                    .fail_arch
                    .as_ref()
                    .and_then(|x| x.is_match(&repo_package.arch).ok())
                    .unwrap_or(false)
                {
                    push = false;
                }

                // Since there are duplicate tree packages (e.g. mesa and
                // mesa-amber), do not push if there is a matching package
                // from tree_vec.
                if tree_vec
                    .iter()
                    .any(|x| x.name == repo_package.name && x.version == repo_package.version)
                {
                    push = false;
                }

                if push {
                    result.push(TreeVsRepo {
                        name: repo_package.name.to_string(),
                        arch: repo_package.arch.to_string(),
                        tree_version: tree_package.version.to_string(),
                        repo_version: repo_package.version.to_string(),
                        compare: match cmp_versions(&tree_package.version, &repo_package.version) {
                            Ordering::Less => DpkgCompare::Less,
                            Ordering::Equal => DpkgCompare::Equal,
                            Ordering::Greater => DpkgCompare::Greeter,
                        },
                    });
                }
            }
        }
    }
    for i in all_no_match {
        let tree = tree_vec
            .iter()
            .find(|x| x.name == i.name)
            .ok_or_else(|| eyre!("Could not find tree version"))?;

        let tree_version = tree.version.to_string();
        let is_noarch = tree.is_noarch;
        let mut repo_not_all_match = repo_vec
            .iter()
            .filter(|x| x.name == i.name && x.arch != "all");

        if is_noarch {
            let repo_pkg = repo_vec
                .iter()
                .find(|x| x.name == i.name && x.arch == "all");

            if let Some(repo_pkg) = repo_pkg {
                let repo_version = repo_pkg.version.to_string();
                result.push(TreeVsRepo {
                    name: i.name.to_string(),
                    arch: "all".to_string(),
                    tree_version: tree_version.clone(),
                    repo_version: repo_version.clone(),
                    compare: match cmp_versions(&tree_version, &repo_version) {
                        Ordering::Less => DpkgCompare::Less,
                        Ordering::Equal => DpkgCompare::Equal,
                        Ordering::Greater => DpkgCompare::Greeter,
                    },
                })
            } else {
                let v = repo_not_all_match
                    .next()
                    .ok_or_else(|| eyre!("repo_not_all_match is empty"))?;

                result.push(TreeVsRepo {
                    name: v.name.to_string(),
                    arch: "all".to_string(),
                    tree_version: tree_version.to_string(),
                    repo_version: v.version.to_string(),
                    compare: match cmp_versions(&tree_version, &v.version) {
                        Ordering::Less => DpkgCompare::Less,
                        Ordering::Equal => DpkgCompare::Equal,
                        Ordering::Greater => DpkgCompare::Greeter,
                    },
                });
            }
        } else {
            for j in repo_not_all_match {
                result.push(TreeVsRepo {
                    name: j.name.to_string(),
                    arch: j.arch.to_string(),
                    tree_version: tree_version.to_string(),
                    repo_version: j.version.to_string(),
                    compare: match oma_apt::util::cmp_versions(&tree_version, &j.version) {
                        std::cmp::Ordering::Less => DpkgCompare::Less,
                        std::cmp::Ordering::Equal => DpkgCompare::Equal,
                        std::cmp::Ordering::Greater => DpkgCompare::Greeter,
                    },
                });
            }
        }
    }

    result.sort_by(|x, y| x.name.cmp(&y.name));

    Ok(result)
}

pub fn result_to_file(result: Vec<TreeVsRepo>, output: String, now_env: PathBuf) -> Result<()> {
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
    std::fs::write(path, file_str)?;

    Ok(())
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
        fail_arch: None,
    }];

    assert!(get_result(repo_vec, tree_vec).unwrap().is_empty());
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
        fail_arch: None,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec).unwrap(),
        vec![TreeVsRepo {
            name: "qaq".to_string(),
            arch: "all".to_string(),
            tree_version: "1:1.0".to_string(),
            repo_version: "1.0".to_string(),
            compare: DpkgCompare::Greeter,
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
        fail_arch: None,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec).unwrap(),
        vec![TreeVsRepo {
            name: "qaq".to_string(),
            arch: "owo".to_string(),
            tree_version: "114514".to_string(),
            repo_version: "114513".to_string(),
            compare: DpkgCompare::Greeter,
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
        fail_arch: None,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec).unwrap(),
        vec![
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "owo".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "114513".to_string(),
                compare: DpkgCompare::Greeter,
            },
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "pwp".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "1.0".to_string(),
                compare: DpkgCompare::Greeter,
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
        fail_arch: None,
    }];

    assert_eq!(
        get_result(repo_vec, tree_vec).unwrap(),
        vec![
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "owo".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "114513".to_string(),
                compare: DpkgCompare::Greeter,
            },
            TreeVsRepo {
                name: "qaq".to_string(),
                arch: "pwp".to_string(),
                tree_version: "114514".to_string(),
                repo_version: "1.0".to_string(),
                compare: DpkgCompare::Greeter,
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
        fail_arch: None,
    }];

    assert!(get_result(repo_vec, tree_vec).unwrap().is_empty())
}

#[test]
fn test_get_result_7() {
    let repo_vec = vec![RepoPackage {
        name: "qaq".to_string(),
        version: "1.0".to_string(),
        arch: "all".to_string(),
    }];
    let tree_vec = vec![
        TreePackage {
            name: "qaq".to_string(),
            version: "1.0".to_string(),
            is_noarch: true,
            fail_arch: None,
        },
        TreePackage {
            name: "qaq".to_string(),
            version: "0.1".to_string(),
            is_noarch: true,
            fail_arch: None,
        },
    ];

    assert!(get_result(repo_vec, tree_vec).unwrap().is_empty())
}
