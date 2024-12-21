mod quest;

use std::error::Error;
use std::time::Duration;
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
    #[arg(
        short,
        long,
        global = true,
        default_value = "./.quests",
        help = "File to source quests from"
    )]
    file: PathBuf,
    #[arg(
        short,
        long,
        global = true,
        default_value = "./.env",
        help = "Load environment variables from file"
    )]
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
            Commands::Go(SendArgs {
                name,
                var,
                header,
                param,
                timeout,
                gzip,
                deflate,
                brotli,
            }) => {
                let quest = questfile
                    .retrieve(&name)
                    .expect("Could not find quest with matching name.");
                let url = questfile
                    .url(quest, var, param)
                    .expect("Could not construct url");
                let headers = questfile.headers(quest, header);
                let client = reqwest::blocking::ClientBuilder::new()
                    .gzip(gzip)
                    .deflate(deflate)
                    .brotli(brotli)
                    .build()
                    .unwrap();

                let mut req = client
                    .request(quest.method.into(), url)
                    .headers(headers)
                    .timeout(Duration::from_secs(timeout));

                if let Some(json) = &quest.json {
                    log::debug!("{:?}", json);
                    req = req
                        .body(json.to_owned())
                        .header("Content-Type", "application/json");
                }
                if let Some(body) = &quest.body {
                    req = req.body(body.to_owned());
                }

                let resp = req.send().unwrap();

                println!("{}", resp.text().unwrap());
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
    Go(SendArgs),
    Ls,
}

#[derive(Clone, Debug, Args)]
struct SendArgs {
    #[arg()]
    name: String,
    #[arg(short, long, value_parser = parse_key_val::<String, String>, help = "Overrides or adds value. Can be used multiple times")]
    var: Vec<(String, String)>,
    #[arg(short = 'H', long, value_parser = parse_key_val::<String, String>, help = "Overrides or adds value. Can be used multiple times")]
    header: Vec<(String, String)>,
    #[arg(short, long, value_parser = parse_key_val::<String, String>, help = "Overrides or adds value. Can be used multiple times")]
    param: Vec<(String, String)>,
    #[arg(short, long, default_value = "30", help = "Timeout seconds")]
    timeout: u64,
    #[arg(long, default_value = "false", help = "Use gzip compression")]
    gzip: bool,
    #[arg(long, default_value = "false", help = "Use deflate compression")]
    deflate: bool,
    #[arg(long, default_value = "false", help = "Use brotli compression")]
    brotli: bool,
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
