use backend::workshop::{parse_workshop_html, extract_workshop_id_from_url};

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}")).expect("fixture missing")
}

#[test]
fn parses_multi_scenario_fixture() {
    let html = read_fixture("workshop_multi.html");
    let parsed = parse_workshop_html(&html).expect("parse failed");

    assert_eq!(parsed.workshop_id, "595F2BF2F44836FB");
    assert_eq!(parsed.scenarios.len(), 2);
    assert!(parsed.scenarios.contains(&"{C5EAD55037EB4751}Missions/RHS_CombatOps_MSV.conf".to_string()));
    assert!(parsed.scenarios.contains(&"{731B585620A3F461}Missions/Coop_CombatOps_Cain_Plus.conf".to_string()));

    assert_eq!(parsed.dependency_urls.len(), 2);
    assert!(parsed.dependency_urls.iter().any(|url| url.contains("5AAAC70D754245DD")));
    assert!(parsed.dependency_urls.iter().any(|url| url.contains("5C9758250C8C56F1")));

    let dep_ids: Vec<String> = parsed
        .dependency_urls
        .iter()
        .filter_map(|url| extract_workshop_id_from_url(url))
        .collect();
    assert!(dep_ids.contains(&"5AAAC70D754245DD".to_string()));
    assert!(dep_ids.contains(&"5C9758250C8C56F1".to_string()));
}

#[test]
fn parses_no_scenario_fixture() {
    let html = read_fixture("workshop_no_scenarios.html");
    let parsed = parse_workshop_html(&html).expect("parse failed");

    assert_eq!(parsed.workshop_id, "1111222233334444");
    assert!(parsed.scenarios.is_empty());
    assert_eq!(parsed.dependency_urls.len(), 1);
    assert!(parsed.dependency_urls[0].contains("AAAAAAAAAAAAAAAA"));
}

#[test]
fn extracts_workshop_id_from_url() {
    let url = "https://reforger.armaplatform.com/workshop/595F2BF2F44836FB-RHS-StatusQuo";
    assert_eq!(extract_workshop_id_from_url(url), Some("595F2BF2F44836FB".to_string()));
}

#[derive(Clone)]
struct MockFetcher;

#[async_trait::async_trait]
impl backend::workshop::WorkshopFetcher for MockFetcher {
    async fn fetch_html(&self, url: &str) -> Result<String, String> {
        match url {
            "https://reforger.armaplatform.com/workshop/ROOT" => Ok(read_fixture("workshop_multi.html")),
            "https://reforger.armaplatform.com/workshop/5AAAC70D754245DD-Some-Mod" => {
                Ok(read_fixture("workshop_dep_5AAA.html"))
            }
            "https://reforger.armaplatform.com/workshop/5C9758250C8C56F1-Other-Mod" => {
                Ok(read_fixture("workshop_dep_5C97.html"))
            }
            _ => Err("unknown url".to_string()),
        }
    }
}

#[tokio::test]
async fn resolves_dependencies_recursively() {
    let resolver = backend::workshop::WorkshopResolver::new(std::sync::Arc::new(MockFetcher));
    let result = resolver
        .resolve("https://reforger.armaplatform.com/workshop/ROOT", 2)
        .await
        .expect("resolve failed");

    assert_eq!(result.root_id, "595F2BF2F44836FB");
    assert_eq!(result.dependency_ids.len(), 2);
    assert!(result.dependency_ids.contains(&"5AAAC70D754245DD".to_string()));
    assert!(result.dependency_ids.contains(&"5C9758250C8C56F1".to_string()));
}
