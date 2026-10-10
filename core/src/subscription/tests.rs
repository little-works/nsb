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
    snapshot_named("source", nodes, groups, rules, final_)
}

fn snapshot_named(
    name: &str,
    nodes: Vec<Value>,
    groups: Vec<Value>,
    rules: Vec<Value>,
    final_: Option<&str>,
) -> RemoteSnapshot {
    RemoteSnapshot {
        name: String::from(name),
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
fn parses_clash_proxies_without_retaining_groups_or_rules() {
    let parsed = parse_remote(
        &remote(RemoteFormat::Clash, RemoteKeepFields::default()),
        "proxies: [{name: node, type: ss, server: example.com, port: 443}]\nproxy-groups: [{name: group, type: select, proxies: [node]}]\nrules: ['DOMAIN,example.com,group']",
        false,
    )
    .unwrap();

    assert_eq!(parsed.proxy_nodes[0]["tag"], "node");
    assert!(parsed.proxy_groups.is_empty());
    assert!(parsed.route_rules.is_empty());
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
    assert_eq!(
        parsed.route.as_ref().unwrap()["rule_set"][0]["outbound"],
        "source:node"
    );
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
fn assembles_multiple_remotes_into_ordered_selectors_and_keeps_route_rule_order() {
    let template = r#"{"outbounds":[{"type":"direct","tag":"direct"},{"type":"selector","tag":"PROXY","outbounds":["manual"]}],"route":{"final":"direct","rules":[{"tag":"template"}]}}"#;
    let first = snapshot_named(
        "first",
        vec![json!({"type":"shadowsocks", "tag":"first:node"})],
        vec![json!({"type":"selector", "tag":"first:group", "outbounds":["first:node"]})],
        vec![json!({"tag":"remote-first"})],
        Some("first:node"),
    );
    let second = snapshot_named(
        "second",
        vec![
            json!({"type":"trojan", "tag":"second:node"}),
            json!({"type":"trojan", "tag":"second:next"}),
        ],
        Vec::new(),
        vec![
            json!({"tag":"remote-second"}),
            json!({"tag":"remote-first"}),
        ],
        Some("second:next"),
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

    assert_eq!(
        tags,
        vec![
            "direct",
            "PROXY",
            "first:node",
            "second:node",
            "second:next",
            "first:group",
            "first",
            "second"
        ]
    );
    assert_eq!(
        config["outbounds"][1]["outbounds"],
        json!(["first", "second"])
    );
    assert_eq!(
        config["outbounds"][6],
        json!({
            "type": "selector",
            "tag": "first",
            "outbounds": ["first:group", "first:node"]
        })
    );
    assert_eq!(
        rules,
        vec!["template", "remote-first", "remote-second", "remote-first"]
    );
    assert_eq!(config["route"]["final"], "second:next");
}

#[test]
fn keeps_empty_remote_selectors_without_direct_fallback() {
    let zero: Value = serde_json::from_str(
        &build_config(r#"{"outbounds":[]}"#, Vec::new(), None).unwrap(),
    )
    .unwrap();
    assert_eq!(
        zero["outbounds"],
        json!([{"type":"selector","tag":"PROXY","outbounds":["direct"]}])
    );

    let single: Value = serde_json::from_str(
        &build_config(
            r#"{"outbounds":[]}"#,
            vec![snapshot_named(
                "empty",
                Vec::new(),
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
        single["outbounds"],
        json!([{"type":"selector","tag":"PROXY","outbounds":[]}])
    );

    let multiple: Value = serde_json::from_str(
        &build_config(
            r#"{"outbounds":[]}"#,
            vec![
                snapshot_named("first", Vec::new(), Vec::new(), Vec::new(), None),
                snapshot_named("second", Vec::new(), Vec::new(), Vec::new(), None),
            ],
            None,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        multiple["outbounds"],
        json!([
            {"type":"selector","tag":"first","outbounds":[]},
            {"type":"selector","tag":"second","outbounds":[]},
            {"type":"selector","tag":"PROXY","outbounds":["first","second"]}
        ])
    );
}

#[test]
fn preserves_proxy_fields_and_removes_invalid_default_for_multiple_remotes() {
    let config: Value = serde_json::from_str(
        &build_config(
            r#"{
                "outbounds": [{
                    "type": "selector",
                    "tag": "PROXY",
                    "outbounds": ["old"],
                    "default": "old",
                    "interrupt_exist_connections": true
                }]
            }"#,
            vec![
                snapshot_named("first", Vec::new(), Vec::new(), Vec::new(), None),
                snapshot_named("second", Vec::new(), Vec::new(), Vec::new(), None),
            ],
            None,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        config["outbounds"][0],
        json!({
            "type": "selector",
            "tag": "PROXY",
            "outbounds": ["first", "second"],
            "interrupt_exist_connections": true
        })
    );
    assert_eq!(
        config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|outbound| outbound["tag"].as_str())
            .collect::<Vec<_>>(),
        vec!["PROXY", "first", "second"]
    );
}

#[test]
fn rejects_multiple_remote_selector_name_collisions() {
    let duplicate = build_config(
        r#"{}"#,
        vec![
            snapshot_named("same", Vec::new(), Vec::new(), Vec::new(), None),
            snapshot_named("same", Vec::new(), Vec::new(), Vec::new(), None),
        ],
        None,
    )
    .unwrap_err();
    assert!(duplicate.contains("duplicate name 'same'"));

    let reserved = build_config(
        r#"{}"#,
        vec![
            snapshot_named("direct", Vec::new(), Vec::new(), Vec::new(), None),
            snapshot_named("other", Vec::new(), Vec::new(), Vec::new(), None),
        ],
        None,
    )
    .unwrap_err();
    assert!(reserved.contains("name 'direct' is reserved"));

    let outbound_conflict = build_config(
        r#"{"outbounds":[{"type":"direct","tag":"same"}]}"#,
        vec![
            snapshot_named("same", Vec::new(), Vec::new(), Vec::new(), None),
            snapshot_named("other", Vec::new(), Vec::new(), Vec::new(), None),
        ],
        None,
    )
    .unwrap_err();
    assert!(outbound_conflict.contains("conflicts with an existing outbound tag"));
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
    let mut first = snapshot_named("first", Vec::new(), Vec::new(), Vec::new(), None);
    first.dns = Some(json!({
        "final": "first",
        "servers": [{"tag": "first"}]
    }));
    first.inbounds = vec![json!({"tag": "first"})];
    first.route = Some(json!({"rule_set": [{"tag": "first"}]}));
    first.experimental = Some(json!({"cache_file": {"enabled": true}}));
    let mut second = snapshot_named("second", Vec::new(), Vec::new(), Vec::new(), None);
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
    assert_eq!(config["experimental"]["cache_file"]["path"], "second.db");
}

#[test]
fn translates_clash_tls_and_transport_fields_into_singbox_outbounds() {
    let parsed = parse_remote(
        &remote(RemoteFormat::Clash, RemoteKeepFields::default()),
        r#"proxies:
  - { name: vless, type: vless, server: example.com, port: 443, uuid: 043b72cb-1464-467c-a2e7-04a4839d0718, alterId: 0, cipher: auto, udp: true, flow: xtls-rprx-vision, encryption: none, tls: true, skip-cert-verify: true, servername: www.cloudflare.com, reality-opts: { public-key: public, short-id: 83726fcc812d }, client-fingerprint: qq, network: tcp }
  - { name: hysteria, type: hysteria2, server: example.com, port: 8443, password: secret, up: 200, down: 200, obfs: salamander, obfs-password: obfs, sni: hysteria.example.com, skip-cert-verify: false }
  - { name: tuic, type: tuic, server: example.com, port: 12862, uuid: 043b72cb-1464-467c-a2e7-04a4839d0718, password: secret, skip-cert-verify: false, sni: tuic.example.com, alpn: [h3], congestion-controller: bbr, udp-relay-mode: native }
  - { name: anytls, type: anytls, server: example.com, port: 443, password: secret, sni: anytls.example.com }
  - { name: vmess, type: vmess, server: example.com, port: 443, uuid: 043b72cb-1464-467c-a2e7-04a4839d0718, alterId: 0, cipher: auto, network: ws, ws-opts: { path: /path, headers: { Host: example.com } } }"#,
        false,
    )
    .unwrap();

    assert_eq!(
        parsed.proxy_nodes[0],
        json!({
            "tag": "vless",
            "type": "vless",
            "server": "example.com",
            "server_port": 443,
            "uuid": "043b72cb-1464-467c-a2e7-04a4839d0718",
            "flow": "xtls-rprx-vision",
            "network": "tcp",
            "tls": {
                "enabled": true,
                "insecure": true,
                "server_name": "www.cloudflare.com",
                "utls": {"enabled": true, "fingerprint": "qq"},
                "reality": {"enabled": true, "public_key": "public", "short_id": "83726fcc812d"}
            }
        })
    );
    assert_eq!(
        parsed.proxy_nodes[1],
        json!({
            "tag": "hysteria",
            "type": "hysteria2",
            "server": "example.com",
            "server_port": 8443,
            "password": "secret",
            "up_mbps": 200,
            "down_mbps": 200,
            "obfs": {"type": "salamander", "password": "obfs"},
            "tls": {"enabled": true, "insecure": false, "server_name": "hysteria.example.com"}
        })
    );
    assert_eq!(
        parsed.proxy_nodes[2],
        json!({
            "tag": "tuic",
            "type": "tuic",
            "server": "example.com",
            "server_port": 12862,
            "uuid": "043b72cb-1464-467c-a2e7-04a4839d0718",
            "password": "secret",
            "congestion_control": "bbr",
            "udp_relay_mode": "native",
            "tls": {"enabled": true, "insecure": false, "server_name": "tuic.example.com", "alpn": ["h3"]}
        })
    );
    assert_eq!(
        parsed.proxy_nodes[3],
        json!({
            "tag": "anytls",
            "type": "anytls",
            "server": "example.com",
            "server_port": 443,
            "password": "secret",
            "tls": {"enabled": true, "server_name": "anytls.example.com"}
        })
    );
    assert_eq!(
        parsed.proxy_nodes[4],
        json!({
            "tag": "vmess",
            "type": "vmess",
            "server": "example.com",
            "server_port": 443,
            "uuid": "043b72cb-1464-467c-a2e7-04a4839d0718",
            "security": "auto",
            "transport": {"type": "ws", "path": "/path", "headers": {"Host": "example.com"}}
        })
    );
}
