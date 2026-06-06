use clap::Parser;
use oui_lookup::lookup;

#[derive(Parser)]
struct Cli {
    addrs: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    for a in &cli.addrs {
        match lookup(a) {
            Some(v) => println!("{a}\t{v}"),
            None => println!("{a}\t(unknown)"),
        }
    }
}
