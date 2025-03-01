// rust/get_search_roots/src/main.rs

use std::env;
use std::path::Path;
use get_search_roots::get_search_roots;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <git_root_or_package_root>", args[0]);
        std::process::exit(1);
    }
    let root = Path::new(&args[1]);
    match get_search_roots(root) {
        Ok(roots) => {
            for dir in roots {
                println!("{}", dir.display());
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}
