use clap::Parser;
use std::io::Write;
use std::{path::Path, process::Command};
use tabled::{object::Segment, Alignment, Modify, Style, Table, Width};

mod pkgversion;
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
    #[clap(short = 'a', long)]
    arch: Vec<String>,
    /// Set mirror.
    #[clap(short = 'm', long, default_value = DEFAULT_URL)]
    mirror: String,
    /// Set branch (retro/non-retro)
    #[clap(short = 'r', long)]
    retro: bool,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let now_env = std::env::current_dir().expect("Cannot get your env!");
    let arch = args.arch;
    let repo_map = repo::get_repo_package_ver_list(&args.mirror, arch, args.retro).unwrap();
    let tree_map = tree::get_tree_package_list(Path::new(&args.tree));
    let result = vs::get_result(repo_map, tree_map);

    if let Some(output) = args.output {
        vs::result_to_file(result, output, now_env);
    } else {
        let mut table = Table::new(result);

        table
            .with(Modify::new(Segment::all()).with(Alignment::left()))
            .with(Modify::new(Segment::all()).with(Width::wrap(30)))
            .with(Modify::new(Segment::all()).with(|s: &str| format!(" {s} ")))
            .with(Style::psql());

        let mut p = Command::new("less");
        p.arg("-R").arg("-c").arg("-S").env("LESSCHARSET", "UTF-8");
        let mut pager_process = p
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("Can not get less stdin!");

        let _ = pager_process
            .stdin
            .as_mut()
            .expect("Can not get less stdin!")
            .write_all(format!("{table}").as_bytes());

        let _ = pager_process.wait();
    }
}
