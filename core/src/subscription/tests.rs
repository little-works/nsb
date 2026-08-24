use serde_json::{Map, Value, json};

use super::parsing::{convert_rule, normalize_proxy};
use crate::SingBoxConfig;

use super::{RemoteSource, build_config, parse_remote, run_finalize_hook};

fn remote() -> RemoteSource {
    RemoteSource {
        name: "A".into(),
        url: "u".into(),
    }
}

#[test]
fn drops_groups_and_rules_when_disabled() {
    let snapshot = parse_remote(
        &remote(),
        "proxies: [{name: n, type: ss, server: s, port: 1}]\nproxy-groups: [{name: g, type: select, proxies: [n]}]\nrules: [MATCH,g]",
        false,
        false,
    )
    .unwrap();

    assert_eq!(snapshot.proxy_nodes.len(), 1);
    assert!(snapshot.proxy_groups.is_empty() && snapshot.route_rules.is_empty());
}

#[test]
fn parses_anytls_clash_proxy() {
    let snapshot = parse_remote(
        &remote(),
        "proxies: [{name: anytls-node, type: anytls, server: example.com, port: 443, password: secret}]",
        false,
        false,
    )
    .unwrap();

    assert!(snapshot.warnings.is_empty());
    assert_eq!(snapshot.proxy_nodes.len(), 1);
    assert_eq!(
        snapshot.proxy_nodes[0],
        json!({
            "tag": "anytls-node",
            "type": "anytls",
            "server": "example.com",
            "server_port": 443,
            "password": "secret",
        })
    );
}

#[test]
fn generated_config_excludes_source_groups_and_rules_when_disabled() {
    let snapshot = parse_remote(
        &remote(),
        r#"{"outbounds":[{"type":"shadowsocks","tag":"node"},{"type":"selector","tag":"source-group","outbounds":["node"]}],"route":{"rules":[{"action":"route","domain":"example.com","outbound":"source-group"}]}}"#,
        false,
        false,
    )
    .unwrap();

    let config: Value = serde_json::from_str(&build_config(vec![snapshot], None).unwrap()).unwrap();
    let outbounds = config["outbounds"].as_array().unwrap();
    let tags: Vec<_> = outbounds
        .iter()
        .filter_map(|outbound| outbound["tag"].as_str())
        .collect();

    assert_eq!(tags, vec!["node", "PROXY", "direct", "block"]);
    assert!(
        config
            .pointer("/route/rules")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .all(|rule| rule["domain"].as_str() != Some("example.com"))
    );
}

#[test]
fn excludes_dev_data_clash_selector_when_disabled() {
    // Derived from gui/dev-data's cached Clash subscription: ChatGPT is a source
    // selector that must not survive when the profile option is unchecked.
    let snapshot = parse_remote(
        &remote(),
        "proxies: [{name: '新加坡-优化-Gemini-GPT', type: vmess, server: example.com, port: 443, uuid: uuid}]\nproxy-groups: [{name: ChatGPT, type: select, proxies: ['新加坡-优化-Gemini-GPT']}]\nrules: ['DOMAIN-SUFFIX,chatgpt.com,ChatGPT']",
        false,
        false,
    )
    .unwrap();

    let config: Value = serde_json::from_str(&build_config(vec![snapshot], None).unwrap()).unwrap();
    assert!(
        config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|outbound| outbound["tag"].as_str() != Some("ChatGPT"))
    );
    assert!(
        config["route"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .all(|rule| rule["outbound"].as_str() != Some("ChatGPT"))
    );
}

#[test]
fn retains_singbox_groups_and_rules_only_when_enabled() {
    let body = r#"{"outbounds":[{"type":"shadowsocks","tag":"node"},{"type":"selector","tag":"group","outbounds":["node"]}],"route":{"rules":[{"action":"route","outbound":"group"}]}}"#;

    let nodes_only = parse_remote(&remote(), body, false, false).unwrap();
    assert_eq!(nodes_only.proxy_nodes.len(), 1);
    assert!(nodes_only.proxy_groups.is_empty());
    assert!(nodes_only.route_rules.is_empty());

    let complete = parse_remote(&remote(), body, false, true).unwrap();
    assert_eq!(complete.proxy_groups[0]["outbounds"], json!(["node"]));
    assert_eq!(complete.route_rules[0]["outbound"], "group");
}

#[test]
fn prefixes_singbox_source_references_for_multiple_remotes() {
    let snapshot = parse_remote(
        &remote(),
        r#"{"outbounds":[{"type":"shadowsocks","tag":"node"},{"type":"selector","tag":"group","outbounds":["node"]}],"route":{"rules":[{"action":"route","outbound":"group"}]}}"#,
        true,
        true,
    )
    .unwrap();

    assert_eq!(snapshot.proxy_nodes[0]["tag"], "A:node");
    assert_eq!(snapshot.proxy_groups[0]["tag"], "A:group");
    assert_eq!(snapshot.proxy_groups[0]["outbounds"], json!(["A:node"]));
    assert_eq!(snapshot.route_rules[0]["outbound"], "A:group");
}

#[test]
fn keeps_and_converts_clash_groups_and_rules() {
    let snapshot = parse_remote(&remote(), "proxies: [{name: n, type: ss, server: s, port: 1}]\nproxy-groups: [{name: g, type: select, proxies: [n]}]\nrules: ['DOMAIN,example.com,g']", false, true).unwrap();

    assert_eq!(snapshot.proxy_groups.len(), 1, "{:?}", snapshot.warnings);
    assert_eq!(snapshot.proxy_groups[0]["type"], "selector");
    assert_eq!(snapshot.route_rules[0]["domain"], json!(["example.com"]));
}

#[test]
fn prefixes_multi_remote_rule_targets() {
    assert_eq!(
        convert_rule(&remote(), &json!("MATCH,g"), true, &mut vec![]).unwrap()["outbound"],
        "A:g"
    );
}

#[test]
fn normalizes_clash_websocket_options() {
    let mut proxy = Map::from_iter([
        (String::from("network"), json!("ws")),
        (
            String::from("ws-opts"),
            json!({"path": "/socket", "headers": {"Host": "example.com"}}),
        ),
    ]);

    normalize_proxy(&mut proxy, "vmess");

    assert!(!proxy.contains_key("ws-opts"));
    assert_eq!(
        proxy.get("transport"),
        Some(&json!({
            "type": "ws",
            "path": "/socket",
            "headers": {"Host": "example.com"},
        }))
    );
}

#[test]
fn passes_core_generated_config_and_remote_snapshot_to_generate_hook() {
    let remote = RemoteSource {
        name: "Primary".into(),
        url: "https://example.com/subscription".into(),
    };
    let snapshot = parse_remote(
        &remote,
        r#"{"outbounds":[{"type":"shadowsocks","tag":"node"}]}"#,
        false,
        false,
    )
    .unwrap();

    let config: Value = serde_json::from_str(
        &build_config(
            vec![snapshot],
            Some(
                "export function onGenerate(input) { input.singbox.hook_remote = input.remote.name; return input.singbox; }",
            ),
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(config["hook_remote"], "Primary");
    assert!(config["outbounds"].is_array());
}

#[test]
fn passes_config_to_finalize_hook() {
    let config = run_finalize_hook(
        "export function onFinalize(input) { input.singbox.finalized = true; return input.singbox; }",
        json!({"outbounds": []}),
    )
    .unwrap();

    assert_eq!(config["finalized"], true);
}

#[test]
fn converts_supported_clash_rules_and_prepends_them_to_template_rules() {
    let snapshot = parse_remote(
        &remote(),
        r#"{"proxies":[],"rules":["DOMAIN,example.com,PROXY","DOMAIN-SUFFIX,example.com,PROXY","DOMAIN-KEYWORD,example,PROXY","IP-CIDR,1.1.1.0/24,PROXY","IP-CIDR6,::1/128,PROXY","PROCESS-NAME,app,PROXY","DST-PORT,443,PROXY","MATCH,PROXY","GEOIP,CN,PROXY"]}"#,
        false,
        true,
    )
    .unwrap();

    assert_eq!(snapshot.route_rules.len(), 8);
    let config: Value = serde_json::from_str(&build_config(vec![snapshot], None).unwrap()).unwrap();
    let rules = config
        .pointer("/route/rules")
        .and_then(Value::as_array)
        .unwrap();
    assert_eq!(rules[0]["domain"], json!(["example.com"]));
    assert_eq!(rules[1]["domain_suffix"], json!(["example.com"]));
    assert_eq!(rules[2]["domain_keyword"], json!(["example"]));
    assert_eq!(rules[3]["ip_cidr"], json!(["1.1.1.0/24", "::1/128"]));
    assert_eq!(rules[4]["process_name"], json!(["app"]));
    assert_eq!(rules[5]["port"], json!(["443"]));
    assert_eq!(rules[6]["outbound"], "source:PROXY");
    assert_eq!(rules[7]["action"], "hijack-dns");

    serde_json::from_value::<SingBoxConfig>(config)
        .expect("generated Clash route rules must deserialize as SingBoxConfig");
}

#[test]
fn merges_consecutive_compatible_subscription_rules_without_duplicate_values() {
    let snapshot = parse_remote(
        &remote(),
        r#"{"proxies":[],"rules":["DOMAIN,example.com,PROXY","DOMAIN,api.example.com,PROXY","DOMAIN,example.com,PROXY"]}"#,
        false,
        true,
    )
    .unwrap();

    let config: Value = serde_json::from_str(&build_config(vec![snapshot], None).unwrap()).unwrap();
    let rules = config["route"]["rules"].as_array().unwrap();
    assert_eq!(
        rules[0]["domain"],
        json!(["example.com", "api.example.com"])
    );
    assert_eq!(rules[0]["action"], "route");
    assert_eq!(rules[0]["outbound"], "source:PROXY");
    assert_eq!(rules[1]["action"], "hijack-dns");

    serde_json::from_value::<SingBoxConfig>(config)
        .expect("merged subscription route rules must deserialize as SingBoxConfig");
}

#[test]
fn does_not_merge_different_matchers_or_constraints() {
    let snapshot = parse_remote(
        &remote(),
        r#"{"outbounds":[],"route":{"rules":[{"action":"route","outbound":"PROXY","domain":["example.com"]},{"action":"route","outbound":"PROXY","domain_suffix":["example.com"]},{"action":"route","outbound":"PROXY","domain":["api.example.com"],"network":"tcp"},{"action":"route","outbound":"PROXY","domain":["www.example.com"]}]}}"#,
        false,
        true,
    )
    .unwrap();

    let config: Value = serde_json::from_str(&build_config(vec![snapshot], None).unwrap()).unwrap();
    let rules = config["route"]["rules"].as_array().unwrap();
    assert_eq!(rules[0]["domain"], json!(["example.com"]));
    assert_eq!(rules[1]["domain_suffix"], json!(["example.com"]));
    assert_eq!(rules[2]["domain"], json!(["api.example.com"]));
    assert_eq!(rules[2]["network"], "tcp");
    assert_eq!(rules[3]["domain"], json!(["www.example.com"]));
}
