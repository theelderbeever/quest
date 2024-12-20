mod quest;

use std::error::Error;
use std::{fs::File, path::PathBuf};

use clap::{Args, Parser, Subcommand};

use quest::QuestFile;

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
    #[arg(short, long, default_value = "./.quests")]
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
            Commands::Send(SendArgs {
                name,
                var,
                header,
                param,
            }) => {
                let quest = questfile
                    .retrieve(&name)
                    .expect("Could not find quest with matching name.");
                let url = questfile
                    .url(quest, var, param)
                    .expect("Could not construct url");
                let headers = questfile.headers(quest, header);
                let client = reqwest::blocking::ClientBuilder::new()
                    .gzip(true)
                    .build()
                    .unwrap();

                let req = client
                    .request(quest.method.into(), url)
                    .headers(headers)
                    .send()
                    .unwrap();

                println!("{}", req.text().unwrap());
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
    Send(SendArgs),
    Ls,
}

#[derive(Clone, Debug, Args)]
struct SendArgs {
    #[arg()]
    name: String,
    #[arg(short, long, value_parser = parse_key_val::<String, String>)]
    var: Vec<(String, String)>,
    #[arg(short = 'H', long, value_parser = parse_key_val::<String, String>)]
    header: Vec<(String, String)>,
    #[arg(short, long, value_parser = parse_key_val::<String, String>)]
    param: Vec<(String, String)>,
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
