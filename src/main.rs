mod quest;

use std::error::Error;
use std::{fs::File, path::PathBuf};

use clap::{Args, Parser, Subcommand};

use quest::{Quest, QuestFile};

fn main() {
    env_logger::init();
    QuestCli::parse().run();
}

fn print_version() -> &'static str {
    Box::leak(format!("v{}", env!("CARGO_PKG_VERSION")).into())
}

#[derive(Clone, Debug, Parser)]
#[command(name = "quest")]
#[command(version = print_version(), about = "Cli for all the http quests you may go on.", long_about = None)]
struct QuestCli {
    #[arg(short, long, default_value = "./quests.yaml")]
    file: PathBuf,
    #[arg(short, long, default_value = "./.env")]
    env: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

impl QuestCli {
    pub fn run(self) {
        log::debug!("{:?}", self);
        if dotenvy::from_path(&self.env).is_ok() {
            log::debug!("Environment loaded from {:?}", self.env);
        }
        let f = File::open(&self.file).expect("Could not open quest file.");

        let questfile: QuestFile = serde_yaml::from_reader(f).expect("Could not parse quest file.");

        match self.command {
            Commands::Fetch(args) | Commands::Get(args) => {
                let quest = questfile
                    .retrieve(&args.name)
                    .expect("Could not find quest with matching name.");
                args.run(quest)
            }

            Commands::Post(args) => {
                let quest = questfile
                    .retrieve(&args.name)
                    .expect("Could not find quest with matching name.");
                args.run(quest)
            }

            Commands::Ls => {
                println!("{:?}", self.file);
                questfile.pretty_print()
            }
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
enum Commands {
    Fetch(Get),
    Get(Get),
    Post(Post),
    Ls,
}

#[derive(Clone, Debug, Args)]
struct Get {
    #[arg()]
    name: String,
    #[arg(short, long, value_parser = parse_key_val::<String, String>)]
    var: Vec<(String, String)>,
    #[arg(short = 'H', long, value_parser = parse_key_val::<String, String>)]
    header: Vec<(String, String)>,
    #[arg(short, long, value_parser = parse_key_val::<String, String>)]
    param: Vec<(String, String)>,
}

impl Get {
    pub fn run(self, quest: Quest) {
        println!(
            "{}",
            quest
                .request(&quest::Method::Get, &self.var, &self.param, &self.header)
                .send()
                .unwrap()
                .text()
                .unwrap()
        );
    }
}

#[derive(Clone, Debug, Args)]
struct Post {
    #[arg()]
    name: String,
    #[arg(short, long, value_parser = parse_key_val::<String, String>)]
    var: Vec<String>,
    #[arg(short = 'H', long, value_parser = parse_key_val::<String, String>)]
    header: Vec<(String, String)>,
    #[arg(short, long, value_parser = parse_key_val::<String, String>)]
    param: Vec<(String, String)>,
    #[arg(short, long)]
    data: Option<String>,
}

impl Post {
    pub fn run(self, quest: Quest) {
        println!("{:?}", self);
        println!("{:?}", quest);
    }
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid key=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}
