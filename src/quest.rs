use std::{
    collections::{BTreeMap, HashMap},
    env::VarError,
};

use colored::{ColoredString, Colorize};
use reqwest::{
    blocking::RequestBuilder,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuestFile {
    quests: Vec<Quest>,
}

impl QuestFile {
    pub fn retrieve(self, name: &str) -> Option<Quest> {
        self.quests.into_iter().find(|q| q.name.eq(name))
    }
    pub fn pretty_print(&self) {
        let fmt_len = self.quests.iter().fold(1, |acc, q| acc.max(q.name.len())) + 4;
        println!(
            "{:<7} {:<fmt_len$} {}",
            "METHOD".bold(),
            "NAME".bold(),
            "VARS".bold()
        );
        self.quests.iter().for_each(|quest| {
            for method in quest.methods.keys() {
                println!(
                    "{:<7} {:<fmt_len$} {}",
                    method.pretty_string(),
                    quest.name,
                    quest
                        .vars(method, &[])
                        .keys()
                        .cloned()
                        .collect::<Vec<String>>()
                        .join(",")
                );
            }
        });
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Quest {
    pub name: String,
    pub url: String,
    #[serde(default)]
    vars: Vec<ConfiguredValue>,
    #[serde(default)]
    headers: Vec<ConfiguredValue>,
    #[serde(default)]
    params: Vec<ConfiguredValue>,
    methods: BTreeMap<Method, Send>,
}

impl Quest {
    pub fn url(&self, method: &Method, extra_vars: &[(String, String)]) -> String {
        let vars = self.vars(method, extra_vars);
        envsubst::substitute(self.url.clone(), &vars).unwrap()
    }

    pub fn vars(&self, method: &Method, extra: &[(String, String)]) -> HashMap<String, String> {
        let mut vars = self.vars.clone();

        if let Some(send) = self.methods.get(method) {
            vars.extend(send.vars.clone());
        }

        let mut vars: Vec<(String, String)> = vars
            .into_iter()
            .map(|v| (v.key(), v.value().expect("Missing environment variable")))
            .collect();

        vars.extend(extra.to_vec());

        log::debug!("vars: {:?}", vars);

        vars.into_iter().collect()
    }
    pub fn params(&self, method: &Method, extra: &[(String, String)]) -> HashMap<String, String> {
        let mut params = self.params.clone();

        if let Some(send) = self.methods.get(method) {
            params.extend(send.params.clone());
        }

        let mut params: Vec<(String, String)> = params
            .into_iter()
            .map(|v| (v.key(), v.value().expect("Missing environment variable")))
            .collect();

        params.extend(extra.to_vec());

        log::debug!("params: {:?}", params);

        params.into_iter().collect()
    }
    pub fn headers(&self, method: &Method, extra: &[(String, String)]) -> HeaderMap {
        let mut headers = self.headers.clone();

        if let Some(send) = self.methods.get(method) {
            headers.extend(send.headers.clone());
        }

        let mut headers: Vec<(String, String)> = headers
            .into_iter()
            .map(|v| (v.key(), v.value().expect("Missing environment variable")))
            .collect();

        headers.extend(extra.to_vec());

        log::debug!("headers: {:?}", headers);

        headers
            .into_iter()
            .map(|(k, v)| {
                (
                    HeaderName::from_lowercase(k.to_lowercase().as_bytes()).unwrap(),
                    HeaderValue::from_str(&v).unwrap(),
                )
            })
            .collect()
    }
    #[allow(clippy::just_underscores_and_digits)]
    pub fn request(
        self,
        method: &Method,
        vars: &[(String, String)],
        params: &[(String, String)],
        headers: &[(String, String)],
    ) -> RequestBuilder {
        log::debug!("{:?}", self);
        let url = self.url(method, vars);
        log::debug!("{method} {url}");
        let params = self.params(method, params);
        let headers = self.headers(method, headers);

        let client = reqwest::blocking::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();
        let url = Url::parse_with_params(&url, params).unwrap();

        match method {
            Method::Get => client.get(url),
            Method::Post => {
                let mut r = client.post(url);
                if let Some(body) = self.methods.get(method).unwrap().body.clone() {
                    log::debug!("{:?}", body);
                    r = r.body(body);
                }
                if let Some(json) = self.methods.get(method).unwrap().json.clone() {
                    log::debug!("{:?}", json);
                    r = r.json(&json);
                }
                r
            } // _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    Get,
    Post,
    // Put,
    // Delete,
    // Head,
}

impl Method {
    pub fn pretty_string(&self) -> ColoredString {
        match self {
            Self::Get => "GET".green(),
            Self::Post => "POST".blue(),
            // Self::Put => "PUT".yellow(),
            // Self::Delete => "DELETE".red(),
            // Self::Head => "HEAD".purple(),
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self).to_lowercase())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Send {
    pub vars: Vec<ConfiguredValue>,
    pub headers: Vec<ConfiguredValue>,
    pub params: Vec<ConfiguredValue>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub json: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ValueFrom {
    env: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfiguredValue {
    key: String,
    value: Option<String>,
    #[serde(rename = "valueFrom")]
    from: Option<ValueFrom>,
}

impl ConfiguredValue {
    pub fn key(&self) -> String {
        self.key.to_owned()
    }
    pub fn value(&self) -> Result<String, VarError> {
        if let Some(v) = &self.value {
            return Ok(v.to_owned());
        }

        if let Some(from) = &self.from {
            return std::env::var(from.env.clone());
        }

        panic!("One of `value` or `valueFrom` must be configured.");
    }
}
