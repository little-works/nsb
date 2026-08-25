use chrono::Local;
use serde_json::{Map, Value, json};

use super::{RemoteFormat, RemoteSnapshot, RemoteSource};

const RESERVED_TAGS: &[&str] = &["PROXY", "direct", "block"];
const HEALTHCHECK_URL: &str = "https://www.gstatic.com/generate_204";

pub fn parse_remote(
    remote: &RemoteSource,
    body: &str,
    multi_remote: bool,
) -> Result<RemoteSnapshot, String> {
    let value: Value = serde_json::from_str(body).or_else(|json_error| {
        serde_yaml::from_str(body).map_err(|yaml_error| {
            format!(
                "Failed to parse Remote {} (JSON: {json_error}; YAML: {yaml_error})",
                remote.name
            )
        })
    })?;
    let mut snapshot = match remote.format {
        RemoteFormat::Clash => parse_clash(remote, value, multi_remote)?,
        RemoteFormat::Singbox => parse_singbox(remote, value, multi_remote)?,
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
        if matches!(kind, "direct" | "block") {
            continue;
        }
        let mut item = normalize_tag(outbound.clone(), remote, multi)?;
        rewrite_source_references(&mut item, remote, multi);
        if remote.keep.groups && matches!(kind, "selector" | "urltest" | "fallback" | "loadbalance")
        {
            groups.push(item);
        } else if remote.keep.nodes
            && !matches!(kind, "selector" | "urltest" | "fallback" | "loadbalance")
        {
            nodes.push(item);
        }
    }
    let rules = if remote.keep.route_rules {
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
    if remote.keep.route_final {
        snapshot.route_final = value
            .pointer("/route/final")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    Ok(snapshot)
}

fn parse_clash(remote: &RemoteSource, value: Value, multi: bool) -> Result<RemoteSnapshot, String> {
    let mut nodes = Vec::new();
    let mut groups = Vec::new();
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
                | "ssr"
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
        rename(&mut item, "cipher", "method");
        normalize_proxy(&mut item, kind);
        item.insert("type".into(), Value::String(kind.into()));
        if remote.keep.nodes {
            nodes.push(normalize_tag(Value::Object(item), remote, multi)?);
        }
    }
    if remote.keep.groups {
        for group in value
            .get("proxy-groups")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            if let Some(group) = convert_group(remote, group, multi, &mut warnings)? {
                groups.push(group);
            }
        }
    }
    let rules = if remote.keep.route_rules {
        value
            .get("rules")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|rule| convert_rule(remote, rule, multi, &mut warnings))
            .collect()
    } else {
        Vec::new()
    };
    Ok(snapshot(remote, nodes, groups, rules, warnings))
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
        warnings,
        updated_at: 0,
    }
}

fn convert_group(
    remote: &RemoteSource,
    group: Value,
    multi: bool,
    warnings: &mut Vec<String>,
) -> Result<Option<Value>, String> {
    let mut item = group.as_object().cloned().ok_or_else(|| {
        format!(
            "Remote {} contains an invalid Clash proxy-group",
            remote.name
        )
    })?;
    let kind = match item
        .remove("type")
        .and_then(|value| value.as_str().map(str::to_owned))
        .as_deref()
    {
        Some("select") => "selector",
        Some("url-test" | "load-balance" | "fallback") => "urltest",
        Some(value) => {
            warnings.push(format!(
                "Skipped unsupported Clash group type {value} in {}",
                remote.name
            ));
            return Ok(None);
        }
        None => {
            warnings.push(format!(
                "Skipped Clash group without type in {}",
                remote.name
            ));
            return Ok(None);
        }
    };
    let members = item.remove("proxies").unwrap_or_else(|| json!([]));
    item.insert("outbounds".into(), rewrite_members(members, remote, multi));
    item.remove("lazy");
    if kind == "urltest" {
        item.entry("url").or_insert_with(|| json!(HEALTHCHECK_URL));
        item.entry("interval").or_insert_with(|| json!("5m"));
        if let Some(Value::Number(number)) = item.get("interval") {
            item.insert("interval".into(), json!(format!("{number}s")));
        }
    }
    item.insert("type".into(), json!(kind));
    normalize_tag(Value::Object(item), remote, multi).map(Some)
}

pub(super) fn convert_rule(
    remote: &RemoteSource,
    value: &Value,
    multi: bool,
    warnings: &mut Vec<String>,
) -> Option<Value> {
    let raw = value.as_str()?;
    let fields: Vec<_> = raw.split(',').map(str::trim).collect();
    if fields.len() < 2 {
        warnings.push(format!(
            "Skipped invalid Clash rule {raw} in {}",
            remote.name
        ));
        return None;
    }
    let (kind, target) = (fields[0].to_ascii_uppercase(), fields.last()?.to_string());
    let outbound = rewrite_target(&target, remote, multi);
    let mut rule = match kind.as_str() {
        // Sing-box route matchers are sequences, including when Clash provides
        // just one value. Keeping that shape makes the generated configuration
        // compatible with the typed SingBoxConfig representation and sing-box.
        "DOMAIN" => json!({"domain": [fields[1]]}),
        "DOMAIN-SUFFIX" => json!({"domain_suffix": [fields[1]]}),
        "DOMAIN-KEYWORD" => json!({"domain_keyword": [fields[1]]}),
        "IP-CIDR" | "IP-CIDR6" => json!({"ip_cidr": [fields[1]]}),
        "PROCESS-NAME" => json!({"process_name": [fields[1]]}),
        "DST-PORT" => json!({"port": [fields[1]]}),
        "MATCH" => json!({}),
        "GEOIP" => {
            warnings.push(format!("Skipped Clash GEOIP rule {raw} in {}", remote.name));
            return None;
        }
        _ => {
            warnings.push(format!(
                "Skipped unsupported Clash rule {raw} in {}",
                remote.name
            ));
            return None;
        }
    };
    rule["action"] = json!("route");
    rule["outbound"] = json!(outbound);
    Some(rule)
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

fn rewrite_members(value: Value, remote: &RemoteSource, multi: bool) -> Value {
    Value::Array(
        value
            .as_array()
            .into_iter()
            .flatten()
            .map(|value| {
                Value::String(rewrite_target(
                    value.as_str().unwrap_or_default(),
                    remote,
                    multi,
                ))
            })
            .collect(),
    )
}

fn rewrite_source_references(value: &mut Value, remote: &RemoteSource, multi: bool) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
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
}

fn rename(item: &mut Map<String, Value>, from: &str, to: &str) {
    if let Some(value) = item.remove(from) {
        item.insert(to.into(), value);
    }
}

pub(super) fn normalize_proxy(proxy: &mut Map<String, Value>, kind: &str) {
    proxy.remove("alterId");
    proxy.remove("udp");
    let ws_opts = proxy
        .remove("ws-opts")
        .and_then(|value| value.as_object().cloned());
    let network = proxy
        .remove("network")
        .and_then(|value| value.as_str().map(str::to_owned));
    let ws_path = proxy.remove("ws-path");
    let ws_headers = proxy.remove("ws-headers");
    if network.as_deref() == Some("ws") {
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
        proxy.insert("transport".into(), Value::Object(transport));
    }
    if let Some(insecure) = proxy.remove("skip-cert-verify") {
        let mut tls = proxy
            .remove("tls")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        tls.insert("enabled".into(), json!(true));
        tls.insert("insecure".into(), insecure);
        proxy.insert("tls".into(), Value::Object(tls));
    }
    if let Some(sni) = proxy.remove("sni") {
        let mut tls = proxy
            .remove("tls")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        tls.insert("enabled".into(), json!(true));
        tls.insert("server_name".into(), sni);
        proxy.insert("tls".into(), Value::Object(tls));
    }
    if kind == "vmess" && proxy.get("method") == Some(&json!("auto")) {
        rename(proxy, "method", "security");
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
