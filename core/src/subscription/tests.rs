use serde_json::{Value, json};

use super::{
    RemoteFormat, RemoteKeepFields, RemoteSnapshot, RemoteSource, build_config, parse_remote,
};

fn remote(format: RemoteFormat, keep: RemoteKeepFields) -> RemoteSource {
    RemoteSource {
        name: String::from("source"),
        url: String::from("https://example.test/subscription"),
        format,
        keep,
    }
}

fn snapshot(
    nodes: Vec<Value>,
    groups: Vec<Value>,
    rules: Vec<Value>,
    final_: Option<&str>,
) -> RemoteSnapshot {
    RemoteSnapshot {
        name: String::from("source"),
        url: String::from("https://example.test/subscription"),
        proxy_nodes: nodes,
        proxy_groups: groups,
        route_rules: rules,
        route_final: final_.map(String::from),
        warnings: Vec::new(),
        updated_at: 0,
    }
}

#[test]
fn parses_clash_only_selected_fields() {
    let keep = RemoteKeepFields {
        nodes: false,
        groups: true,
        route_final: false,
        route_rules: true,
    };
    let parsed = parse_remote(&remote(RemoteFormat::Clash, keep), "proxies: [{name: node, type: ss, server: example.com, port: 443}]\nproxy-groups: [{name: group, type: select, proxies: [node]}]\nrules: ['DOMAIN,example.com,group']", false).unwrap();

    assert!(parsed.proxy_nodes.is_empty());
    assert_eq!(parsed.proxy_groups[0]["tag"], "group");
    assert_eq!(parsed.route_rules[0]["outbound"], "group");
}

#[test]
fn parses_singbox_only_selected_fields() {
    let keep = RemoteKeepFields {
        nodes: true,
        groups: false,
        route_final: true,
        route_rules: false,
    };
    let parsed = parse_remote(&remote(RemoteFormat::Singbox, keep), r#"{"outbounds":[{"type":"shadowsocks","tag":"node"},{"type":"selector","tag":"group","outbounds":["node"]}],"route":{"final":"node","rules":[{"action":"route","outbound":"node"}]}}"#, false).unwrap();

    assert_eq!(parsed.proxy_nodes[0]["tag"], "node");
    assert!(parsed.proxy_groups.is_empty());
    assert!(parsed.route_rules.is_empty());
    assert_eq!(parsed.route_final.as_deref(), Some("node"));
}

#[test]
fn uses_declared_remote_format_instead_of_detecting_document_shape() {
    let error = parse_remote(
        &remote(RemoteFormat::Singbox, RemoteKeepFields::default()),
        "proxies: [{name: node, type: ss, server: example.com, port: 443}]",
        false,
    )
    .unwrap_err();

    assert!(error.contains("missing outbounds"));
}

#[test]
fn assembles_template_first_and_keeps_route_rule_order() {
    let template = r#"{"outbounds":[{"type":"direct","tag":"direct"},{"type":"selector","tag":"PROXY","outbounds":["manual"]}],"route":{"final":"direct","rules":[{"tag":"template"}]}}"#;
    let first = snapshot(
        vec![json!({"type":"shadowsocks", "tag":"node"})],
        Vec::new(),
        vec![json!({"tag":"remote-first"})],
        Some("node"),
    );
    let second = snapshot(
        vec![
            json!({"type":"trojan", "tag":"node"}),
            json!({"type":"trojan", "tag":"next"}),
        ],
        Vec::new(),
        vec![
            json!({"tag":"remote-second"}),
            json!({"tag":"remote-first"}),
        ],
        Some("next"),
    );

    let config: Value =
        serde_json::from_str(&build_config(template, vec![first, second], None).unwrap()).unwrap();
    let tags: Vec<_> = config["outbounds"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|outbound| outbound["tag"].as_str())
        .collect();
    let rules: Vec<_> = config["route"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|rule| rule["tag"].as_str())
        .collect();

    assert_eq!(tags, vec!["direct", "PROXY", "node", "next"]);
    assert_eq!(
        config["outbounds"][1]["outbounds"],
        json!(["manual", "node", "next"])
    );
    assert_eq!(
        rules,
        vec!["template", "remote-first", "remote-second", "remote-first"]
    );
    assert_eq!(config["route"]["final"], "next");
}

#[test]
fn creates_proxy_selector_when_template_has_none() {
    let config: Value = serde_json::from_str(
        &build_config(
            r#"{"outbounds":[]}"#,
            vec![snapshot(
                vec![json!({"type":"shadowsocks", "tag":"node"})],
                Vec::new(),
                Vec::new(),
                None,
            )],
            None,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        config["outbounds"][1],
        json!({"type":"selector", "tag":"PROXY", "outbounds":["node"]})
    );
}
