use args::Command;
use clap::Parser;

mod args;
mod cassette;
mod commands;
mod misc;

fn main() {
    let args = args::Args::parse();

    match args.subcommand {
        Command::Decode(decode) => commands::decode::decode(decode),
    }
}
