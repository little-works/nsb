use chrono::Local;
use jsonc_parser::{ParseOptions, parse_to_serde_value};
use serde_json::{Map, Value, json};

use super::{RemoteFormat, RemoteSnapshot, RemoteSource};
use super::links::parse_links;

const RESERVED_TAGS: &[&str] = &["PROXY", "direct", "block"];

pub fn parse_remote(
    remote: &RemoteSource,
    body: &str,
    multi_remote: bool,
) -> Result<RemoteSnapshot, String> {
    let mut snapshot = match remote.format {
        RemoteFormat::Clash => {
            let value: Value = serde_json::from_str(body).or_else(|json_error| {
                serde_yaml::from_str(body).map_err(|yaml_error| {
                    format!(
                        "Failed to parse Remote {} (JSON: {json_error}; YAML: {yaml_error})",
                        remote.name
                    )
                })
            })?;
            parse_clash(remote, value, multi_remote)?
        }
        RemoteFormat::Singbox => {
            let options = ParseOptions {
                allow_comments: true,
                allow_loose_object_property_names: false,
                allow_trailing_commas: true,
                allow_missing_commas: false,
                allow_single_quoted_strings: false,
                allow_hexadecimal_numbers: false,
                allow_unary_plus_numbers: false,
            };
            let value = parse_to_serde_value::<Value>(body, &options).map_err(|error| {
                format!("Failed to parse Remote {} as JSONC: {error}", remote.name)
            })?;
            parse_singbox(remote, value, multi_remote)?
        }
        RemoteFormat::Links => parse_links(remote, body, multi_remote)?,
    };
    snapshot.updated_at = u64::try_from(Local::now().timestamp()).unwrap_or_default();
    Ok(snapshot)
}

fn parse_singbox(
    remote: &RemoteSource,
    value: Value,
    multi: bool,
) -> Result<RemoteSnapshot, String> {
    let outbounds = value
        .get("outbounds")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Remote {} is missing outbounds", remote.name))?;
    let mut nodes = Vec::new();
    let mut groups = Vec::new();
    for outbound in outbounds {
        let kind = outbound
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !remote.keep.outbounds {
            continue;
        }
        let mut item = normalize_tag(outbound.clone(), remote, multi)?;
        rewrite_source_references(&mut item, remote, multi);
        if matches!(kind, "selector" | "urltest" | "fallback" | "loadbalance") {
            groups.push(item);
        } else {
            nodes.push(item);
        }
    }
    let rules = if remote.keep.route.rules {
        value
            .pointer("/route/rules")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|mut rule| {
                rewrite_source_references(&mut rule, remote, multi);
                rule
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut snapshot = snapshot(remote, nodes, groups, rules, Vec::new());
    if remote.keep.route.final_ {
        snapshot.route_final = value
            .pointer("/route/final")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    snapshot.dns = preserved_dns(&value, remote, multi);
    if remote.keep.inbounds {
        snapshot.inbounds = preserved_array(&value, "/inbounds", remote, multi);
    }
    snapshot.route = preserved_route(&value, remote, multi);
    snapshot.experimental = preserved_experimental(&value, remote, multi);
    Ok(snapshot)
}

fn selected_fields(source: &Map<String, Value>, fields: &[(&str, bool)]) -> Map<String, Value> {
    fields
        .iter()
        .filter_map(|(key, enabled)| {
            enabled
                .then(|| {
                    source
                        .get(*key)
                        .cloned()
                        .map(|value| (String::from(*key), value))
                })
                .flatten()
        })
        .collect()
}

fn preserved_dns(value: &Value, remote: &RemoteSource, multi: bool) -> Option<Value> {
    let source = value.get("dns")?.as_object()?;
    let keep = &remote.keep.dns;
    let mut selected = selected_fields(
        source,
        &[
            ("servers", keep.servers),
            ("rules", keep.rules),
            ("final", keep.final_),
            ("strategy", keep.strategy),
            ("disable_cache", keep.disable_cache),
            ("disable_expire", keep.disable_expire),
            ("independent_cache", keep.independent_cache),
            ("cache_capacity", keep.cache_capacity),
            ("timeout", keep.timeout),
            ("reverse_mapping", keep.reverse_mapping),
            ("client_subnet", keep.client_subnet),
            ("fakeip", keep.fakeip),
        ],
    );
    if let Some(source_optimistic) = source.get("optimistic").and_then(Value::as_object) {
        let optimistic = selected_fields(
            source_optimistic,
            &[
                ("enabled", keep.optimistic.enabled),
                ("timeout", keep.optimistic.timeout),
            ],
        );
        if !optimistic.is_empty() {
            selected.insert(String::from("optimistic"), Value::Object(optimistic));
        }
    }
    preserved_object(selected, remote, multi)
}

fn preserved_route(value: &Value, remote: &RemoteSource, multi: bool) -> Option<Value> {
    let source = value.get("route")?.as_object()?;
    let keep = &remote.keep.route;
    let selected = selected_fields(
        source,
        &[
            ("rule_set", keep.rule_set),
            ("auto_detect_interface", keep.auto_detect_interface),
            ("override_android_vpn", keep.override_android_vpn),
            ("default_interface", keep.default_interface),
            ("default_mark", keep.default_mark),
            ("find_process", keep.find_process),
            ("find_neighbor", keep.find_neighbor),
            ("dhcp_lease_files", keep.dhcp_lease_files),
            ("default_http_client", keep.default_http_client),
            ("default_domain_resolver", keep.default_domain_resolver),
            ("default_network_strategy", keep.default_network_strategy),
            ("default_network_type", keep.default_network_type),
            (
                "default_fallback_network_type",
                keep.default_fallback_network_type,
            ),
            ("default_fallback_delay", keep.default_fallback_delay),
        ],
    );
    preserved_object(selected, remote, multi)
}

fn preserved_experimental(value: &Value, remote: &RemoteSource, multi: bool) -> Option<Value> {
    let source = value.get("experimental")?.as_object()?;
    let mut selected = Map::new();
    if let Some(cache_file) = source.get("cache_file").and_then(Value::as_object) {
        let keep = &remote.keep.experimental.cache_file;
        let cache_file = selected_fields(
            cache_file,
            &[
                ("enabled", keep.enabled),
                ("path", keep.path),
                ("cache_id", keep.cache_id),
                ("store_fakeip", keep.store_fakeip),
                ("store_rdrc", keep.store_rdrc),
                ("rdrc_timeout", keep.rdrc_timeout),
                ("store_dns", keep.store_dns),
            ],
        );
        if !cache_file.is_empty() {
            selected.insert(String::from("cache_file"), Value::Object(cache_file));
        }
    }
    if let Some(clash_api) = source.get("clash_api").and_then(Value::as_object) {
        let keep = &remote.keep.experimental.clash_api;
        let clash_api = selected_fields(
            clash_api,
            &[
                ("external_controller", keep.external_controller),
                ("external_ui", keep.external_ui),
                ("external_ui_download_url", keep.external_ui_download_url),
                (
                    "external_ui_download_detour",
                    keep.external_ui_download_detour,
                ),
                ("secret", keep.secret),
                ("default_mode", keep.default_mode),
                (
                    "access_control_allow_origin",
                    keep.access_control_allow_origin,
                ),
                (
                    "access_control_allow_private_network",
                    keep.access_control_allow_private_network,
                ),
                ("store_mode", keep.store_mode),
                ("store_selected", keep.store_selected),
                ("store_fakeip", keep.store_fakeip),
                ("cache_file", keep.cache_file),
                ("cache_id", keep.cache_id),
            ],
        );
        if !clash_api.is_empty() {
            selected.insert(String::from("clash_api"), Value::Object(clash_api));
        }
    }
    if remote.keep.experimental.v2ray_api
        && let Some(v2ray_api) = source.get("v2ray_api")
    {
        selected.insert(String::from("v2ray_api"), v2ray_api.clone());
    }
    preserved_object(selected, remote, multi)
}

fn preserved_object(
    selected: Map<String, Value>,
    remote: &RemoteSource,
    multi: bool,
) -> Option<Value> {
    if selected.is_empty() {
        return None;
    }
    let mut value = Value::Object(selected);
    rewrite_source_references(&mut value, remote, multi);
    Some(value)
}

fn preserved_array(value: &Value, pointer: &str, remote: &RemoteSource, multi: bool) -> Vec<Value> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|mut item| {
            rewrite_source_references(&mut item, remote, multi);
            item
        })
        .collect()
}

fn parse_clash(remote: &RemoteSource, value: Value, multi: bool) -> Result<RemoteSnapshot, String> {
    let mut nodes = Vec::new();
    let mut warnings = Vec::new();
    for proxy in value
        .get("proxies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let mut item = proxy
            .as_object()
            .cloned()
            .ok_or_else(|| format!("Remote {} contains an invalid Clash proxy", remote.name))?;
        let kind = item
            .remove("type")
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        let kind = match kind.as_str() {
            "ss" => "shadowsocks",
            "socks5" => "socks",
            value => value,
        };
        if !matches!(
            kind,
            "shadowsocks"
                | "vmess"
                | "vless"
                | "trojan"
                | "anytls"
                | "hysteria2"
                | "tuic"
                | "socks"
                | "http"
        ) {
            warnings.push(format!(
                "Skipped unsupported Clash proxy type {kind} in {}",
                remote.name
            ));
            continue;
        }
        rename(&mut item, "port", "server_port");
        normalize_proxy(&mut item, kind);
        item.insert("type".into(), Value::String(kind.into()));
        if remote.keep.clash.proxies {
            nodes.push(normalize_tag(Value::Object(item), remote, multi)?);
        }
    }
    Ok(snapshot(remote, nodes, Vec::new(), Vec::new(), warnings))
}

fn snapshot(
    remote: &RemoteSource,
    proxy_nodes: Vec<Value>,
    proxy_groups: Vec<Value>,
    route_rules: Vec<Value>,
    warnings: Vec<String>,
) -> RemoteSnapshot {
    RemoteSnapshot {
        name: remote.name.clone(),
        url: remote.url.clone(),
        proxy_nodes,
        proxy_groups,
        route_rules,
        route_final: None,
        dns: None,
        inbounds: Vec::new(),
        route: None,
        experimental: None,
        warnings,
        updated_at: 0,
    }
}

fn rewrite_target(target: &str, remote: &RemoteSource, multi: bool) -> String {
    match target {
        value if value.eq_ignore_ascii_case("DIRECT") => "direct".into(),
        "REJECT" | "REJECT-DROP" => "block".into(),
        value if RESERVED_TAGS.contains(&value) => {
            let tag = if multi {
                format!("{}:{value}", remote.name)
            } else {
                value.into()
            };
            format!("source:{tag}")
        }
        value if multi => format!("{}:{value}", remote.name),
        value => value.into(),
    }
}

fn rewrite_source_references(value: &mut Value, remote: &RemoteSource, multi: bool) {
    match value {
        Value::Object(object) => {
            for key in ["outbounds", "outbound", "detour"] {
                if let Some(entry) = object.get_mut(key) {
                    match entry {
                        Value::String(tag) => *tag = rewrite_target(tag, remote, multi),
                        Value::Array(tags) => {
                            for entry in tags {
                                if let Value::String(tag) = entry {
                                    *tag = rewrite_target(tag, remote, multi);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            for entry in object.values_mut() {
                rewrite_source_references(entry, remote, multi);
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrite_source_references(item, remote, multi);
            }
        }
        _ => {}
    }
}

fn rename(item: &mut Map<String, Value>, from: &str, to: &str) {
    if let Some(value) = item.remove(from) {
        item.insert(to.into(), value);
    }
}

/// Translates Clash-only proxy fields into their sing-box equivalents.
///
/// Everything sing-box would reject as unknown, misspelled or mistyped is either
/// relocated into the nested `tls`/`transport`/`obfs` objects or dropped, so the
/// generated outbound only ever contains fields accepted by the kernel.
pub(super) fn normalize_proxy(proxy: &mut Map<String, Value>, kind: &str) {
    proxy.remove("alterId");
    proxy.remove("udp");
    // Clash-only carry-overs without a sing-box counterpart.
    proxy.remove("encryption");
    proxy.remove("fingerprint");
    match kind {
        "shadowsocks" => rename(proxy, "cipher", "method"),
        "vmess" => rename(proxy, "cipher", "security"),
        "vless" => {
            proxy.remove("cipher");
        }
        "hysteria2" => normalize_hysteria2(proxy),
        "tuic" => {
            rename(proxy, "congestion-controller", "congestion_control");
            rename(proxy, "udp-relay-mode", "udp_relay_mode");
        }
        _ => {}
    }
    normalize_transport(proxy);
    normalize_tls(proxy, kind);
}

fn normalize_transport(proxy: &mut Map<String, Value>) {
    let network = proxy
        .remove("network")
        .and_then(|value| value.as_str().map(str::to_ascii_lowercase));
    let ws_opts = proxy
        .remove("ws-opts")
        .and_then(|value| value.as_object().cloned());
    let ws_path = proxy.remove("ws-path");
    let ws_headers = proxy.remove("ws-headers");
    let http_opts = proxy
        .remove("http-opts")
        .and_then(|value| value.as_object().cloned());
    let h2_opts = proxy
        .remove("h2-opts")
        .and_then(|value| value.as_object().cloned());
    let grpc_opts = proxy
        .remove("grpc-opts")
        .and_then(|value| value.as_object().cloned());
    let transport = match network.as_deref() {
        Some("ws") => {
            let mut transport = Map::from_iter([(String::from("type"), json!("ws"))]);
            if let Some(path) = ws_opts
                .as_ref()
                .and_then(|options| options.get("path"))
                .cloned()
                .or(ws_path)
            {
                transport.insert("path".into(), path);
            }
            if let Some(headers) = ws_opts
                .as_ref()
                .and_then(|options| options.get("headers"))
                .cloned()
                .or(ws_headers)
            {
                transport.insert("headers".into(), headers);
            }
            Some(transport)
        }
        Some("http") => Some(http_transport(http_opts.as_ref())),
        Some("h2") => Some(http_transport(h2_opts.as_ref())),
        Some("grpc") => {
            let mut transport = Map::from_iter([(String::from("type"), json!("grpc"))]);
            if let Some(service_name) = grpc_opts
                .as_ref()
                .and_then(|options| options.get("grpc-service-name"))
                .cloned()
            {
                transport.insert("service_name".into(), service_name);
            }
            Some(transport)
        }
        Some(value @ ("tcp" | "udp")) => {
            proxy.insert("network".into(), json!(value));
            None
        }
        _ => None,
    };
    if let Some(transport) = transport {
        proxy.insert("transport".into(), Value::Object(transport));
    }
}

fn http_transport(options: Option<&Map<String, Value>>) -> Map<String, Value> {
    let mut transport = Map::from_iter([(String::from("type"), json!("http"))]);
    let Some(options) = options else {
        return transport;
    };
    if let Some(host) = options.get("host").cloned() {
        transport.insert("host".into(), host);
    }
    if let Some(path) = options.get("path").cloned() {
        let path = match path {
            Value::Array(items) => items.into_iter().next().unwrap_or_default(),
            value => value,
        };
        transport.insert("path".into(), path);
    }
    if let Some(method) = options.get("method").cloned() {
        transport.insert("method".into(), method);
    }
    if let Some(headers) = options.get("headers").cloned() {
        transport.insert("headers".into(), headers);
    }
    transport
}

fn normalize_tls(proxy: &mut Map<String, Value>, kind: &str) {
    let mut tls = match proxy.remove("tls") {
        Some(Value::Object(object)) => object,
        Some(Value::Bool(enabled)) => Map::from_iter([(String::from("enabled"), json!(enabled))]),
        _ => Map::new(),
    };
    if let Some(insecure) = proxy.remove("skip-cert-verify") {
        tls.insert(String::from("insecure"), insecure);
        tls.entry(String::from("enabled")).or_insert(json!(true));
    }
    if let Some(server_name) = proxy.remove("sni").or_else(|| proxy.remove("servername")) {
        tls.insert(String::from("server_name"), server_name);
        tls.entry(String::from("enabled")).or_insert(json!(true));
    }
    if let Some(alpn) = proxy.remove("alpn") {
        tls.insert(String::from("alpn"), alpn);
        if !tls.contains_key("enabled") {
            tls.insert(String::from("enabled"), json!(true));
        }
    }
    if let Some(fingerprint) = proxy.remove("client-fingerprint") {
        let utls = tls
            .entry(String::from("utls"))
            .or_insert_with(|| json!({ "enabled": true }));
        if let Some(utls) = utls.as_object_mut() {
            utls.insert(String::from("enabled"), json!(true));
            utls.insert(String::from("fingerprint"), fingerprint);
        }
        if !tls.contains_key("enabled") {
            tls.insert(String::from("enabled"), json!(true));
        }
    }
    if let Some(reality) = proxy
        .remove("reality-opts")
        .and_then(|value| value.as_object().cloned())
    {
        let mut options = Map::from_iter([(String::from("enabled"), json!(true))]);
        if let Some(public_key) = reality.get("public-key") {
            options.insert(String::from("public_key"), public_key.clone());
        }
        if let Some(short_id) = reality.get("short-id") {
            options.insert(String::from("short_id"), short_id.clone());
        }
        tls.insert(String::from("reality"), Value::Object(options));
        // sing-box refuses to initialize a Reality client without uTLS.
        tls.entry(String::from("utls"))
            .or_insert_with(|| json!({ "enabled": true }));
        if !tls.contains_key("enabled") {
            tls.insert(String::from("enabled"), json!(true));
        }
    }
    if matches!(kind, "hysteria2" | "tuic" | "anytls") {
        tls.insert(String::from("enabled"), json!(true));
    }
    if !tls.is_empty() {
        proxy.insert(String::from("tls"), Value::Object(tls));
    }
}

fn normalize_hysteria2(proxy: &mut Map<String, Value>) {
    for (from, to) in [("up", "up_mbps"), ("down", "down_mbps")] {
        if let Some(value) = proxy.remove(from)
            && let Some(mbps) = bandwidth_to_mbps(&value)
        {
            proxy.insert(String::from(to), json!(mbps));
        }
    }
    let password = proxy.remove("obfs-password");
    let min_packet_size = proxy.remove("obfs-min-packet-size");
    let max_packet_size = proxy.remove("obfs-max-packet-size");
    let Some(obfs) = proxy.remove("obfs") else {
        return;
    };
    let mut obfs = match obfs {
        Value::Object(object) => object,
        Value::String(obfs_type) => {
            Map::from_iter([(String::from("type"), Value::String(obfs_type))])
        }
        _ => return,
    };
    if let Some(password) = password {
        obfs.insert(String::from("password"), password);
    }
    if let Some(min_packet_size) = min_packet_size {
        obfs.insert(String::from("min_packet_size"), min_packet_size);
    }
    if let Some(max_packet_size) = max_packet_size {
        obfs.insert(String::from("max_packet_size"), max_packet_size);
    }
    proxy.insert(String::from("obfs"), Value::Object(obfs));
}

/// Reads a Clash bandwidth value such as `30`, `30 Mbps` or `"30mbps"` as Mbps.
fn bandwidth_to_mbps(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64().or_else(|| {
            number
                .as_f64()
                .filter(|value| *value >= 0.0)
                .map(|value| value as u64)
        }),
        Value::String(text) => {
            let digits: String = text
                .trim()
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse().ok()
        }
        _ => None,
    }
}

fn normalize_tag(mut value: Value, remote: &RemoteSource, multi: bool) -> Result<Value, String> {
    let item = value
        .as_object_mut()
        .ok_or_else(|| "outbound must be an object".to_string())?;
    let tag = item
        .get("tag")
        .or_else(|| item.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("outbound in Remote {} is missing name", remote.name))?;
    let mut next = if multi {
        format!("{}:{tag}", remote.name)
    } else {
        tag.into()
    };
    if RESERVED_TAGS.contains(&tag) {
        next = format!("source:{next}");
    }
    item.remove("name");
    item.insert("tag".into(), json!(next));
    Ok(value)
}
