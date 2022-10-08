use clap::Parser;
use env_logger::Env;
use tempdir::{TemporaryDirectory, clean_directories};

/// A program to create a temporary directory. The directory
/// deletes itself after the specified amount of time
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    action: Actions,
}

#[derive(clap::Subcommand, Debug)]
enum Actions {
    Create {
        /// Name of the tempory folder to create
        #[clap(short, long, value_parser)]
        name: String,

        /// Duration the directory will live.
        /// Examples: 1d, 4w, 8m
        #[clap(short, long, value_parser)]
        duration: String,
    },
    Clean,
}

fn main() {
    // Enable Logging
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "trace")
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    // Parse command line arguments
    let args = Args::parse();

    match args.action {
        Actions::Create { name, duration } => {
            let tempdir = TemporaryDirectory::new(name, duration).unwrap();
            tempdir.create();
        }
        Actions::Clean => {
            clean_directories();
        }
    }
}
