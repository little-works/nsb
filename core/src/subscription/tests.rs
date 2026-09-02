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
        dns: None,
        inbounds: Vec::new(),
        route: None,
        experimental: None,
        warnings: Vec::new(),
        updated_at: 0,
    }
}

#[test]
fn parses_clash_proxies_proxy_groups_and_rules() {
    let mut keep = RemoteKeepFields::default();
    keep.clash.rules = true;
    let parsed = parse_remote(&remote(RemoteFormat::Clash, keep), "proxies: [{name: node, type: ss, server: example.com, port: 443}]\nproxy-groups: [{name: group, type: select, proxies: [node]}]\nrules: ['DOMAIN,example.com,group']", false).unwrap();

    assert_eq!(parsed.proxy_nodes[0]["tag"], "node");
    assert_eq!(parsed.proxy_groups[0]["tag"], "group");
    assert_eq!(parsed.route_rules[0]["outbound"], "group");
}

#[test]
fn controls_each_clash_configuration_block_independently() {
    let body = "proxies: [{name: node, type: ss, server: example.com, port: 443}]\nproxy-groups: [{name: group, type: select, proxies: [node]}]\nrules: ['DOMAIN,example.com,group']";
    let mut keep = RemoteKeepFields::default();
    keep.clash.proxies = false;
    keep.clash.rules = true;
    let without_proxies =
        parse_remote(&remote(RemoteFormat::Clash, keep), body, false).unwrap();

    assert!(without_proxies.proxy_nodes.is_empty());
    assert_eq!(without_proxies.proxy_groups[0]["tag"], "group");
    assert_eq!(without_proxies.route_rules[0]["outbound"], "group");

    let mut keep = RemoteKeepFields::default();
    keep.clash.proxy_groups = false;
    let without_groups_or_rules =
        parse_remote(&remote(RemoteFormat::Clash, keep), body, false).unwrap();

    assert_eq!(without_groups_or_rules.proxy_nodes[0]["tag"], "node");
    assert!(without_groups_or_rules.proxy_groups.is_empty());
    assert!(without_groups_or_rules.route_rules.is_empty());
}

#[test]
fn filters_clash_rules_whose_outbound_block_is_not_retained() {
    let body = "proxies: [{name: node, type: ss, server: example.com, port: 443}]\nproxy-groups: [{name: group, type: select, proxies: [node]}]\nrules: ['DOMAIN,node.example,node', 'DOMAIN,group.example,group', 'DOMAIN,direct.example,DIRECT', 'DOMAIN,reject.example,REJECT']";
    let mut keep = RemoteKeepFields::default();
    keep.clash.proxies = false;
    keep.clash.rules = true;
    let without_proxies =
        parse_remote(&remote(RemoteFormat::Clash, keep), body, true).unwrap();
    let targets: Vec<_> = without_proxies
        .route_rules
        .iter()
        .filter_map(|rule| rule["outbound"].as_str())
        .collect();

    assert_eq!(targets, vec!["source:group", "direct", "block"]);

    let mut keep = RemoteKeepFields::default();
    keep.clash.proxy_groups = false;
    keep.clash.rules = true;
    let without_groups =
        parse_remote(&remote(RemoteFormat::Clash, keep), body, true).unwrap();
    let targets: Vec<_> = without_groups
        .route_rules
        .iter()
        .filter_map(|rule| rule["outbound"].as_str())
        .collect();

    assert_eq!(targets, vec!["source:node", "direct", "block"]);
}

#[test]
fn parses_singbox_outbounds_as_nodes_and_groups() {
    let mut keep = RemoteKeepFields::default();
    keep.route.final_ = true;
    let parsed = parse_remote(&remote(RemoteFormat::Singbox, keep), r#"{"outbounds":[{"type":"shadowsocks","tag":"node"},{"type":"selector","tag":"group","outbounds":["node"]}],"route":{"final":"node","rules":[{"action":"route","outbound":"node"}]}}"#, false).unwrap();

    assert_eq!(parsed.proxy_nodes[0]["tag"], "node");
    assert_eq!(parsed.proxy_groups[0]["tag"], "group");
    assert!(parsed.route_rules.is_empty());
    assert_eq!(parsed.route_final.as_deref(), Some("node"));
}

#[test]
fn preserves_singbox_direct_and_block_outbounds() {
    let parsed = parse_remote(
        &remote(RemoteFormat::Singbox, RemoteKeepFields::default()),
        r#"{
            "outbounds": [
                {
                    "tag": "qunhe-out",
                    "type": "direct",
                    "domain_resolver": "dns-device-local"
                },
                {"tag": "qunhe-block", "type": "block"}
            ]
        }"#,
        false,
    )
    .unwrap();

    assert_eq!(parsed.proxy_nodes.len(), 2);
    assert_eq!(
        parsed.proxy_nodes[0],
        json!({
            "tag": "qunhe-out",
            "type": "direct",
            "domain_resolver": "dns-device-local"
        })
    );
    assert_eq!(
        parsed.proxy_nodes[1],
        json!({"tag": "qunhe-block", "type": "block"})
    );
}

#[test]
fn skips_singbox_nodes_and_groups_when_outbounds_are_disabled() {
    let keep = RemoteKeepFields {
        outbounds: false,
        ..RemoteKeepFields::default()
    };
    let parsed = parse_remote(
        &remote(RemoteFormat::Singbox, keep),
        r#"{
            "outbounds": [
                {"type": "shadowsocks", "tag": "node"},
                {"type": "selector", "tag": "group", "outbounds": ["node"]}
            ]
        }"#,
        false,
    )
    .unwrap();

    assert!(parsed.proxy_nodes.is_empty());
    assert!(parsed.proxy_groups.is_empty());
}

#[test]
fn uses_declared_remote_format_instead_of_detecting_document_shape() {
    let error = parse_remote(
        &remote(RemoteFormat::Singbox, RemoteKeepFields::default()),
        "proxies: [{name: node, type: ss, server: example.com, port: 443}]",
        false,
    )
    .unwrap_err();

    assert!(error.contains("Failed to parse Remote source as JSONC"));
}

#[test]
fn parses_singbox_jsonc_comments_and_trailing_commas() {
    let parsed = parse_remote(
        &remote(RemoteFormat::Singbox, RemoteKeepFields::default()),
        r#"{
            // A line comment.
            "outbounds": [
                {"type": "shadowsocks", "tag": "node"},
            ],
            /* A block comment. */
        }"#,
        false,
    )
    .unwrap();

    assert_eq!(parsed.proxy_nodes[0]["tag"], "node");
}

#[test]
fn rejects_singbox_syntax_outside_jsonc() {
    for body in [
        r#"{"outbounds": [{"type": "direct"} {"type": "block"}]}"#,
        r#"{'outbounds': []}"#,
        r#"{outbounds: []}"#,
        r#"{"outbounds": [], "value": 0x10}"#,
        r#"{"outbounds": [], "value": +1}"#,
    ] {
        let error = parse_remote(
            &remote(RemoteFormat::Singbox, RemoteKeepFields::default()),
            body,
            false,
        )
        .unwrap_err();

        assert!(error.contains("Failed to parse Remote source as JSONC"));
    }
}

#[test]
fn keeps_selected_singbox_configuration_fragments_and_rewrites_references() {
    let mut keep = RemoteKeepFields::default();
    keep.outbounds = false;
    keep.inbounds = true;
    keep.dns.servers = true;
    keep.dns.optimistic.timeout = true;
    keep.route.rule_set = true;
    keep.route.default_mark = true;
    keep.experimental.cache_file.enabled = true;
    keep.experimental.clash_api.secret = true;
    keep.experimental.v2ray_api = true;
    let parsed = parse_remote(
        &remote(RemoteFormat::Singbox, keep),
        r#"{
            "outbounds": [],
            "dns": {
                "servers": [{"tag": "remote", "detour": "node"}],
                "strategy": "prefer_ipv4",
                "optimistic": {"enabled": true, "timeout": "1s"}
            },
            "inbounds": [{"type": "mixed", "tag": "mixed", "detour": "node"}],
            "route": {
                "rule_set": [{"tag": "rules", "outbound": "node"}],
                "default_mark": 255,
                "find_process": true
            },
            "experimental": {
                "cache_file": {"enabled": true},
                "clash_api": {
                    "external_controller": "127.0.0.1:9090",
                    "secret": "selected"
                },
                "v2ray_api": {"listen": "127.0.0.1:8080", "detour": "node"}
            }
        }"#,
        true,
    )
    .unwrap();

    assert_eq!(
        parsed.dns.as_ref().unwrap()["servers"][0]["detour"],
        "source:node"
    );
    assert_eq!(parsed.inbounds[0]["detour"], "source:node");
    assert_eq!(parsed.route.as_ref().unwrap()["rule_set"][0]["outbound"], "source:node");
    assert_eq!(parsed.route.as_ref().unwrap()["default_mark"], 255);
    assert!(parsed.route.as_ref().unwrap().get("find_process").is_none());
    assert_eq!(parsed.dns.as_ref().unwrap()["optimistic"]["timeout"], "1s");
    assert!(parsed.dns.as_ref().unwrap().get("strategy").is_none());
    assert_eq!(
        parsed.experimental.as_ref().unwrap()["v2ray_api"]["detour"],
        "source:node"
    );
    assert_eq!(
        parsed.experimental.as_ref().unwrap()["cache_file"]["enabled"],
        true
    );
    assert_eq!(
        parsed.experimental.as_ref().unwrap()["clash_api"]["secret"],
        "selected"
    );
    assert!(
        parsed.experimental.as_ref().unwrap()["clash_api"]
            .get("external_controller")
            .is_none()
    );
}

#[test]
fn leaves_disabled_singbox_configuration_fragments_empty() {
    let parsed = parse_remote(
        &remote(RemoteFormat::Singbox, RemoteKeepFields::default()),
        r#"{
            "outbounds": [],
            "dns": {},
            "inbounds": [{}],
            "route": {"rule_set": [{}]},
            "experimental": {}
        }"#,
        false,
    )
    .unwrap();

    assert!(parsed.dns.is_none());
    assert!(parsed.inbounds.is_empty());
    assert!(parsed.route.is_none());
    assert!(parsed.experimental.is_none());
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

#[test]
fn deduplicates_outbounds_by_tag_or_untagged_value_and_excludes_direct_block_from_proxy() {
    let template = r#"{
        "outbounds": [
            {"type": "direct", "tag": "direct"},
            {"type": "direct", "tag": "qunhe-out", "domain_resolver": "template"}
        ]
    }"#;
    let config: Value = serde_json::from_str(
        &build_config(
            template,
            vec![snapshot(
                vec![
                    json!({"type": "direct", "tag": "direct"}),
                    json!({"type": "direct", "tag": "qunhe-out", "domain_resolver": "remote"}),
                    json!({"type": "block", "tag": "qunhe-block"}),
                    json!({"type": "shadowsocks", "tag": "node"}),
                    json!({"type": "direct", "domain_resolver": "dns-device-local"}),
                    serde_json::from_str::<Value>(
                        r#"{"domain_resolver":"dns-device-local","type":"direct"}"#,
                    )
                    .unwrap(),
                    json!({"type": "direct", "domain_resolver": "other-dns"}),
                ],
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
        config["outbounds"],
        json!([
            {"type": "direct", "tag": "direct"},
            {"type": "direct", "tag": "qunhe-out", "domain_resolver": "remote"},
            {"type": "block", "tag": "qunhe-block"},
            {"type": "shadowsocks", "tag": "node"},
            {"type": "direct", "domain_resolver": "dns-device-local"},
            {"type": "direct", "domain_resolver": "other-dns"},
            {"type": "selector", "tag": "PROXY", "outbounds": ["node"]}
        ])
    );
}

#[test]
fn recursively_merges_preserved_fields_in_remote_order_and_appends_arrays() {
    let template = r#"{
        "outbounds": [],
        "dns": {"final": "template", "servers": [{"tag": "template"}]},
        "inbounds": [{"tag": "template"}],
        "experimental": {"cache_file": {"enabled": false, "path": "template.db"}}
    }"#;
    let mut first = snapshot(Vec::new(), Vec::new(), Vec::new(), None);
    first.dns = Some(json!({
        "final": "first",
        "servers": [{"tag": "first"}]
    }));
    first.inbounds = vec![json!({"tag": "first"})];
    first.route = Some(json!({"rule_set": [{"tag": "first"}]}));
    first.experimental = Some(json!({"cache_file": {"enabled": true}}));
    let mut second = snapshot(Vec::new(), Vec::new(), Vec::new(), None);
    second.dns = Some(json!({
        "final": "second",
        "servers": [{"tag": "second"}]
    }));
    second.inbounds = vec![json!({"tag": "second"})];
    second.route = Some(json!({"rule_set": [{"tag": "second"}]}));
    second.experimental = Some(json!({"cache_file": {"path": "second.db"}}));

    let config: Value =
        serde_json::from_str(&build_config(template, vec![first, second], None).unwrap()).unwrap();

    assert_eq!(config["dns"]["final"], "second");
    assert_eq!(
        config["dns"]["servers"],
        json!([{"tag": "template"}, {"tag": "first"}, {"tag": "second"}])
    );
    assert_eq!(
        config["inbounds"],
        json!([{"tag": "template"}, {"tag": "first"}, {"tag": "second"}])
    );
    assert_eq!(
        config["route"]["rule_set"],
        json!([{"tag": "first"}, {"tag": "second"}])
    );
    assert_eq!(config["experimental"]["cache_file"]["enabled"], true);
    assert_eq!(
        config["experimental"]["cache_file"]["path"],
        "second.db"
    );
}
