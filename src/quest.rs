use std::{collections::HashMap, env::VarError};

use colored::{ColoredString, Colorize};
use itertools::*;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum QuestError {
    #[error("No quest named `{0}` found.")]
    MissingQuest(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("Could not substitute variables")]
    FailedToSubstituteVariables(#[from] envsubst::Error),
    #[error("Could not construct url. Check the url and params provided.")]
    MissingConfiguredEnvironmentVar(#[from] VarError),
    #[error("Could not construct url. Check the url and params provided.")]
    InvalidUrl(#[from] url::ParseError),
    #[error("A global or quest specific url must be configured.")]
    MissingUrl,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ConfiguredKeyValue {
    Value {
        name: String,
        value: String,
    },
    ValueFromEnv {
        name: String,
        #[serde(rename = "valueFromEnv")]
        value_from_env: String,
    },
}

impl ConfiguredKeyValue {
    pub fn name(&self) -> String {
        match self {
            Self::Value { name, .. } => name.to_owned(),
            Self::ValueFromEnv { name, .. } => name.to_owned(),
        }
    }
    pub fn value(&self) -> Result<String, VarError> {
        match self {
            Self::Value { value, .. } => Ok(value.to_string()),
            Self::ValueFromEnv { value_from_env, .. } => std::env::var(value_from_env),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct QuestFile {
    headers: Vec<ConfiguredKeyValue>,
    vars: Vec<ConfiguredKeyValue>,
    params: Vec<ConfiguredKeyValue>,
    quests: Vec<Quest>,
}

impl QuestFile {
    pub fn retrieve(&self, name: &str) -> Result<&Quest, QuestError> {
        self.quests
            .iter()
            .find(|q| q.name.eq(name))
            .ok_or(QuestError::MissingQuest(name.to_string()))
    }
    pub fn url(
        &self,
        quest: &Quest,
        vars: Vec<(String, String)>,
        params: Vec<(String, String)>,
    ) -> Result<Url, QuestError> {
        let params = self.params(quest, params);
        let vars = self.vars(quest, vars);

        let url = envsubst::substitute(quest.url.clone(), &vars)?;
        log::debug!("{url}");

        Ok(Url::parse_with_params(&url, params)?)
    }
    pub fn vars(&self, quest: &Quest, vars: Vec<(String, String)>) -> HashMap<String, String> {
        self.vars
            .iter()
            .chain(quest.vars.iter())
            .map(|var| (var.name().to_string(), var.value().unwrap()))
            .chain(vars)
            .collect()
    }
    pub fn params(&self, quest: &Quest, params: Vec<(String, String)>) -> Vec<(String, String)> {
        self.params
            .iter()
            .chain(quest.params.iter())
            .map(|param| (param.name().to_string(), param.value().unwrap()))
            .chain(params)
            .collect()
    }

    pub fn headers(&self, quest: &Quest, headers: Vec<(String, String)>) -> HeaderMap {
        self.headers
            .iter()
            .chain(quest.headers.iter())
            .map(|var| (var.name(), var.value().unwrap()))
            .chain(headers)
            .map(|(name, var)| {
                (
                    HeaderName::from_lowercase(name.to_lowercase().as_bytes()).unwrap(),
                    HeaderValue::from_str(&var).unwrap(),
                )
            })
            .collect()
    }
    #[allow(unstable_name_collisions)]
    pub fn pretty_print(&self) {
        let fmt_len = self.quests.iter().fold(1, |acc, q| acc.max(q.name.len())) + 4;
        println!(
            "{:<7} {:<fmt_len$} {}",
            "METHOD".bold(),
            "NAME".bold(),
            "VARS".bold()
        );
        self.quests.iter().for_each(|quest| {
            println!(
                "{:<7} {:<fmt_len$} {}",
                quest.method.pretty_string(),
                quest.name,
                self.vars(quest, Vec::new())
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .intersperse(", ".to_string())
                    .collect::<String>()
            );
        });
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Quest {
    pub name: String,
    pub method: Method,
    pub url: String,
    #[serde(default)]
    pub vars: Vec<ConfiguredKeyValue>,
    #[serde(default)]
    pub headers: Vec<ConfiguredKeyValue>,
    #[serde(default)]
    pub params: Vec<ConfiguredKeyValue>,
    pub json: Option<String>,
    pub body: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Patch,
}

impl Method {
    pub fn pretty_string(&self) -> ColoredString {
        match self {
            Self::Get => "GET".green(),
            Self::Post => "POST".blue(),
            Self::Put => "PUT".yellow(),
            Self::Delete => "DELETE".red(),
            Self::Head => "HEAD".purple(),
            Self::Patch => "PATCH".cyan(),
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self).to_lowercase())
    }
}

impl From<Method> for reqwest::Method {
    fn from(value: Method) -> Self {
        match value {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
            Method::Head => reqwest::Method::HEAD,
            Method::Patch => reqwest::Method::PATCH,
        }
    }
}
