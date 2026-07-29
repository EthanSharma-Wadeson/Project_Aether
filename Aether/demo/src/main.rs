//! Enterprise Agent Spend Control — CLI entry point.

mod enterprise_demo;

fn main() {
    enterprise_demo::cli::run(std::env::args().skip(1).collect());
}
