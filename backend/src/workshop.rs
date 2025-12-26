use serde::{Deserialize, Serialize};
use scraper::{Html, Selector};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

const WORKSHOP_BASE_URL: &str = "https://reforger.armaplatform.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopResolveRequest {
    pub url: String,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopResolveResult {
    pub root_id: String,
    pub root_url: String,
    pub scenarios: Vec<String>,
    pub dependency_ids: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkshopRootPage {
    pub workshop_id: String,
    pub dependency_urls: Vec<String>,
}

#[async_trait::async_trait]
pub trait WorkshopFetcher: Send + Sync {
    async fn fetch_html(&self, url: &str) -> Result<String, String>;
}

#[derive(Clone)]
pub struct WorkshopResolver {
    fetcher: Arc<dyn WorkshopFetcher>,
}

impl WorkshopResolver {
    pub fn new(fetcher: Arc<dyn WorkshopFetcher>) -> Self {
        Self { fetcher }
    }

    pub async fn resolve(
        &self,
        url: &str,
        max_depth: usize,
    ) -> Result<WorkshopResolveResult, String> {
        let root_id = extract_workshop_id_from_url(url)
            .ok_or_else(|| "failed to extract workshop id from url".to_string())?;

        let root_html = self.fetcher.fetch_html(url).await?;
        let root_page = parse_root_page(&root_html, Some(&root_id))?;

        let scenarios_url = format!("{url}/scenarios");
        let scenarios_html = self.fetcher.fetch_html(&scenarios_url).await?;
        let scenarios = parse_scenarios_page(&scenarios_html);

        let mut dependency_ids = Vec::new();
        let mut errors = Vec::new();
        let mut visited_ids = HashSet::new();
        let mut visited_urls = HashSet::new();

        visited_ids.insert(root_id.clone());
        visited_urls.insert(url.to_string());

        if max_depth > 0 {
            let mut queue = VecDeque::new();
            for dep_url in root_page.dependency_urls.iter() {
                queue.push_back((dep_url.clone(), 1usize));
            }

            while let Some((dep_url, depth)) = queue.pop_front() {
                if depth > max_depth {
                    continue;
                }
                if visited_urls.contains(&dep_url) {
                    continue;
                }
                visited_urls.insert(dep_url.clone());

                let dep_id_hint = extract_workshop_id_from_url(&dep_url);

                let dep_html = match self.fetcher.fetch_html(&dep_url).await {
                    Ok(html) => html,
                    Err(err) => {
                        errors.push(format!("failed to fetch dependency {dep_url}: {err}"));
                        continue;
                    }
                };

                let dep_page = match parse_root_page(&dep_html, dep_id_hint.as_deref()) {
                    Ok(page) => page,
                    Err(err) => {
                        errors.push(format!("failed to parse dependency {dep_url}: {err}"));
                        continue;
                    }
                };

                if visited_ids.insert(dep_page.workshop_id.clone()) {
                    dependency_ids.push(dep_page.workshop_id.clone());
                }

                if depth < max_depth {
                    for next_url in dep_page.dependency_urls.iter() {
                        if !visited_urls.contains(next_url) {
                            queue.push_back((next_url.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        Ok(WorkshopResolveResult {
            root_id,
            root_url: url.to_string(),
            scenarios,
            dependency_ids,
            errors,
        })
    }
}

pub struct ReqwestFetcher {
    client: reqwest::Client,
}

impl ReqwestFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl WorkshopFetcher for ReqwestFetcher {
    async fn fetch_html(&self, url: &str) -> Result<String, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| format!("request failed: {err}"))?;

        if !response.status().is_success() {
            return Err(format!("request failed: status {}", response.status()));
        }

        response
            .text()
            .await
            .map_err(|err| format!("failed to read response: {err}"))
    }
}

pub fn parse_root_page(html: &str, expected_id: Option<&str>) -> Result<WorkshopRootPage, String> {
    let document = Html::parse_document(html);

    let mut workshop_id = expected_id.map(|value| value.to_string());
    let mut dependencies = Vec::new();

    if let Some(value) = extract_embedded_json(&document) {
        if workshop_id.is_none() {
            workshop_id = extract_string(&value, &["workshopId", "id"]);
        }
        dependencies = extract_string_list(&value, &["dependencies"]);
    }

    if workshop_id.is_none() {
        workshop_id = extract_workshop_id_from_html(html);
    }

    if workshop_id.is_none() {
        workshop_id = extract_workshop_id_from_data_props(&document);
    }

    if dependencies.is_empty() {
        dependencies = extract_dependency_urls(&document);
    }

    let workshop_id = workshop_id.ok_or_else(|| "workshop id not found".to_string())?;
    let dependency_urls = normalize_dependency_urls(dependencies);

    Ok(WorkshopRootPage {
        workshop_id,
        dependency_urls,
    })
}

pub fn extract_workshop_id_from_url(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"/workshop/([A-F0-9]{16})").ok()?;
    re.captures(url)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

pub fn extract_workshop_id_from_html(html: &str) -> Option<String> {
    let re = regex::Regex::new(r"\bID\s+([A-F0-9]{16})\b").ok()?;
    re.captures(html)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

pub fn parse_scenarios_page(html: &str) -> Vec<String> {
    if !html.contains("Scenario ID") {
        return Vec::new();
    }

    dedupe_preserve_order(extract_scenarios_from_html(html))
}

fn extract_embedded_json(document: &Html) -> Option<serde_json::Value> {
    let selector = Selector::parse("script#__WORKSHOP_STATE__").ok()?;
    let script = document.select(&selector).next()?;
    let text = script.text().collect::<String>();
    serde_json::from_str(text.trim()).ok()
}

fn extract_workshop_id_from_data_props(document: &Html) -> Option<String> {
    let selector = Selector::parse("[data-props]").ok()?;
    for node in document.select(&selector) {
        if let Some(props) = node.value().attr("data-props") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(props) {
                if let Some(id) = extract_string(&value, &["workshopId", "id"]) {
                    return Some(id);
                }
            }
        }
    }
    None
}

fn extract_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value.get(*key).and_then(|v| v.as_str()) {
            return Some(found.to_string());
        }
    }
    None
}

fn extract_string_list(value: &serde_json::Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(list) = value.get(*key).and_then(|v| v.as_array()) {
            return list
                .iter()
                .filter_map(|entry| entry.as_str().map(|s| s.to_string()))
                .collect();
        }
    }
    Vec::new()
}

fn extract_scenarios_from_html(html: &str) -> Vec<String> {
    let mut scenarios = Vec::new();
    let re = regex::Regex::new(r#"\{[A-F0-9]{16}\}Missions/[^\s"<>]+\.conf"#)
        .expect("scenario regex");

    for caps in re.captures_iter(html) {
        if let Some(value) = caps.get(0) {
            push_unique(&mut scenarios, value.as_str().to_string());
        }
    }

    scenarios
}

fn extract_dependency_urls(document: &Html) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(section_urls) = extract_dependency_urls_from_section(document) {
        urls = section_urls;
    }

    if urls.is_empty() {
        let selector = Selector::parse(r#"a[href*="/workshop/"]"#).expect("link selector");
        for link in document.select(&selector) {
            if let Some(href) = link.value().attr("href") {
                push_unique(&mut urls, href.to_string());
            }
        }
    }
    urls
}

fn extract_dependency_urls_from_section(document: &Html) -> Option<Vec<String>> {
    let section_selector = Selector::parse("section, div").ok()?;
    let link_selector = Selector::parse(r#"a[href*="/workshop/"]"#).ok()?;

    for node in document.select(&section_selector) {
        let text = node.text().collect::<String>();
        if text.contains("Dependencies") {
            let mut urls = Vec::new();
            for link in node.select(&link_selector) {
                if let Some(href) = link.value().attr("href") {
                    push_unique(&mut urls, href.to_string());
                }
            }
            if !urls.is_empty() {
                return Some(urls);
            }
        }
    }
    None
}

fn normalize_dependency_urls(urls: Vec<String>) -> Vec<String> {
    urls.into_iter()
        .map(|url| {
            if url.starts_with("http://") || url.starts_with("https://") {
                url
            } else if url.starts_with('/') {
                format!("{WORKSHOP_BASE_URL}{url}")
            } else {
                format!("{WORKSHOP_BASE_URL}/{url}")
            }
        })
        .collect()
}

fn dedupe_preserve_order(mut values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
    values
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}
