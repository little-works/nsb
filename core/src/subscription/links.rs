use std::collections::HashSet;

use base64::{Engine, engine::general_purpose};
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value, json};
use url::Url;

use super::{RemoteSnapshot, RemoteSource};

const RESERVED_TAGS: &[&str] = &["PROXY", "direct", "block"];

#[derive(Debug, Clone, Copy)]
enum LinkError {
    Unsupported,
    Invalid,
}

pub(super) fn parse_links(
    remote: &RemoteSource,
    body: &str,
    multi: bool,
) -> Result<RemoteSnapshot, String> {
    let body = decode_body(body);
    let mut nodes = Vec::new();
    let mut warnings = Vec::new();
    let mut used_tags = HashSet::new();
    let mut valid_links = 0usize;

    for (line_number, line) in body.lines().enumerate() {
        let link = line.trim();
        if link.is_empty() {
            continue;
        }
        match parse_link(link, remote, multi, &mut used_tags) {
            Ok(node) => {
                valid_links += 1;
                if remote.keep.outbounds {
                    nodes.push(node);
                }
            }
            Err(LinkError::Unsupported) => warnings.push(format!(
                "Skipped unsupported link on line {}",
                line_number + 1
            )),
            Err(LinkError::Invalid) => warnings.push(format!(
                "Skipped invalid link on line {}",
                line_number + 1
            )),
        }
    }

    if valid_links == 0 {
        return Err(String::from("Remote contains no valid supported links"));
    }

    Ok(RemoteSnapshot {
        name: remote.name.clone(),
        url: remote.url.clone(),
        proxy_nodes: nodes,
        proxy_groups: Vec::new(),
        route_rules: Vec::new(),
        route_final: None,
        dns: None,
        inbounds: Vec::new(),
        route: None,
        experimental: None,
        warnings,
        updated_at: 0,
    })
}

fn parse_link(
    link: &str,
    remote: &RemoteSource,
    multi: bool,
    used_tags: &mut HashSet<String>,
) -> Result<Value, LinkError> {
    let url = Url::parse(link).map_err(|_| LinkError::Invalid)?;
    let scheme = url.scheme().to_ascii_lowercase();
    let (host, port) = server_address(&url, &scheme)?;
    let mut outbound = match scheme.as_str() {
        "vless" => parse_vless(&url, host, port)?,
        "hysteria2" | "hy2" => parse_hysteria2(&url, host, port)?,
        "anytls" => parse_anytls(&url, host, port)?,
        "tuic" => parse_tuic(&url, host, port)?,
        _ => return Err(LinkError::Unsupported),
    };

    let fragment = url
        .fragment()
        .map(decode_component)
        .transpose()?
        .unwrap_or_default();
    let base_tag = if fragment.trim().is_empty() {
        fallback_tag(&scheme, &url, port)
    } else {
        clean_tag(&fragment)
    };
    let tag = unique_tag(&base_tag, remote, multi, used_tags);
    outbound.insert(String::from("tag"), Value::String(tag));
    Ok(Value::Object(outbound))
}

fn parse_vless(url: &Url, host: String, port: u16) -> Result<Map<String, Value>, LinkError> {
    let uuid = first_query(url, &["uuid"])
        .or_else(|| decode_component(url.username()).ok())
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Invalid)?;
    let mut outbound = base_outbound("vless", host, port);
    outbound.insert(String::from("uuid"), Value::String(uuid));
    if let Some(flow) = first_query(url, &["flow"]).filter(|value| !value.is_empty()) {
        outbound.insert(String::from("flow"), Value::String(flow));
    }
    add_transport(&mut outbound, url)?;
    if first_query(url, &["mode"])
        .is_some_and(|mode| matches!(mode.to_ascii_lowercase().as_str(), "multi" | "multiplex"))
    {
        outbound.insert(String::from("multiplex"), json!({ "enabled": true }));
    }
    let security = first_query(url, &["security"]);
    let reality = security
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("reality"));
    if reality && first_query(url, &["pbk", "public-key"]).is_none() {
        return Err(LinkError::Invalid);
    }
    if let Some(tls) = tls_config(url, security.as_deref(), false, reality)? {
        outbound.insert(String::from("tls"), tls);
    }
    Ok(outbound)
}

fn parse_hysteria2(url: &Url, host: String, port: u16) -> Result<Map<String, Value>, LinkError> {
    let password = first_query(url, &["password"])
        .or_else(|| hysteria2_userinfo_password(url))
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Invalid)?;
    let mut outbound = base_outbound("hysteria2", host, port);
    outbound.insert(String::from("password"), Value::String(password));
    if let Some(obfs_type) = first_query(url, &["obfs"]).filter(|value| {
        !value.is_empty() && !value.eq_ignore_ascii_case("none")
    }) {
        let mut obfs = Map::from_iter([(String::from("type"), Value::String(obfs_type))]);
        if let Some(password) = first_query(url, &["obfs-password"]) {
            obfs.insert(String::from("password"), Value::String(password));
        }
        outbound.insert(String::from("obfs"), Value::Object(obfs));
    }
    outbound.insert(
        String::from("tls"),
        tls_config(url, None, true, false)?.unwrap_or_else(|| json!({ "enabled": true })),
    );
    Ok(outbound)
}

fn parse_anytls(url: &Url, host: String, port: u16) -> Result<Map<String, Value>, LinkError> {
    let password = first_query(url, &["password"])
        .or_else(|| userinfo_password(url))
        .or_else(|| decode_component(url.username()).ok())
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Invalid)?;
    let mut outbound = base_outbound("anytls", host, port);
    outbound.insert(String::from("password"), Value::String(password));
    outbound.insert(
        String::from("tls"),
        tls_config(url, None, true, false)?.unwrap_or_else(|| json!({ "enabled": true })),
    );
    Ok(outbound)
}

fn parse_tuic(url: &Url, host: String, port: u16) -> Result<Map<String, Value>, LinkError> {
    let uuid = decode_component(url.username())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Invalid)?;
    let password = first_query(url, &["password"])
        .or_else(|| userinfo_password(url))
        .filter(|value| !value.is_empty())
        .ok_or(LinkError::Invalid)?;
    let mut outbound = base_outbound("tuic", host, port);
    outbound.insert(String::from("uuid"), Value::String(uuid));
    outbound.insert(String::from("password"), Value::String(password));
    if let Some(value) = first_query(url, &["congestion_control", "congestion-controller"]) {
        outbound.insert(String::from("congestion_control"), Value::String(value));
    }
    if let Some(value) = first_query(url, &["udp_relay_mode", "udp-relay-mode"]) {
        outbound.insert(String::from("udp_relay_mode"), Value::String(value));
    }
    outbound.insert(
        String::from("tls"),
        tls_config(url, None, true, false)?.unwrap_or_else(|| json!({ "enabled": true })),
    );
    Ok(outbound)
}

fn base_outbound(kind: &str, host: String, port: u16) -> Map<String, Value> {
    Map::from_iter([
        (String::from("type"), Value::String(kind.into())),
        (String::from("server"), Value::String(host)),
        (String::from("server_port"), json!(port)),
    ])
}

fn add_transport(outbound: &mut Map<String, Value>, url: &Url) -> Result<(), LinkError> {
    let Some(kind) = first_query(url, &["type", "transport"]) else {
        return Ok(());
    };
    let kind = kind.to_ascii_lowercase();
    match kind.as_str() {
        "tcp" | "udp" => {
            outbound.insert(String::from("network"), Value::String(kind));
        }
        "ws" | "http" | "h2" | "grpc" | "quic" | "httpupgrade" => {
            let transport_kind = if kind == "h2" { "http" } else { kind.as_str() };
            let mut transport = Map::from_iter([(
                String::from("type"),
                Value::String(transport_kind.into()),
            )]);
            if let Some(path) = first_query(url, &["path"]) {
                transport.insert(String::from("path"), Value::String(path));
            }
            if let Some(host) = first_query(url, &["host", "authority"]) {
                if transport_kind == "ws" {
                    transport.insert(String::from("headers"), json!({ "Host": host }));
                } else if transport_kind == "http" {
                    transport.insert(String::from("host"), json!([host]));
                } else if transport_kind == "httpupgrade" {
                    transport.insert(String::from("host"), Value::String(host));
                }
            }
            if kind == "grpc"
                && let Some(service_name) = first_query(url, &["serviceName", "service-name", "service_name"])
            {
                transport.insert(String::from("service_name"), Value::String(service_name));
            }
            outbound.insert(String::from("transport"), Value::Object(transport));
        }
        _ => return Err(LinkError::Invalid),
    }
    Ok(())
}

fn tls_config(
    url: &Url,
    security: Option<&str>,
    default_enabled: bool,
    reality: bool,
) -> Result<Option<Value>, LinkError> {
    let sni = first_query(url, &["sni", "servername", "server_name"]);
    let insecure = first_query(url, &["insecure", "allow_insecure", "skip-cert-verify"])
        .and_then(|value| parse_bool(&value));
    let fingerprint = first_query(url, &["fp", "client-fingerprint"]);
    let public_key = first_query(url, &["pbk", "public-key"]);
    let short_id = first_query(url, &["sid", "short-id"]);
    let alpn = query_list(url, "alpn");
    let security_none = security.is_some_and(|value| value.eq_ignore_ascii_case("none"));
    let enabled = if security_none {
        false
    } else {
        default_enabled || security.is_some_and(|value| {
            matches!(value.to_ascii_lowercase().as_str(), "tls" | "reality")
        }) || reality
    };
    if !enabled && sni.is_none() && insecure.is_none() && fingerprint.is_none() && !reality {
        return Ok(None);
    }

    let mut tls = Map::from_iter([(String::from("enabled"), Value::Bool(enabled))]);
    if let Some(sni) = sni {
        tls.insert(String::from("server_name"), Value::String(sni));
    }
    if let Some(insecure) = insecure {
        tls.insert(String::from("insecure"), Value::Bool(insecure));
    }
    if !alpn.is_empty() {
        tls.insert(
            String::from("alpn"),
            Value::Array(alpn.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(fingerprint) = fingerprint {
        tls.insert(
            String::from("utls"),
            json!({ "enabled": true, "fingerprint": fingerprint }),
        );
    } else if reality {
        tls.insert(String::from("utls"), json!({ "enabled": true }));
    }
    if reality || public_key.is_some() || short_id.is_some() {
        let mut reality_options = Map::from_iter([(String::from("enabled"), Value::Bool(true))]);
        if let Some(public_key) = public_key {
            reality_options.insert(String::from("public_key"), Value::String(public_key));
        }
        if let Some(short_id) = short_id {
            reality_options.insert(String::from("short_id"), Value::String(short_id));
        }
        // `spx` (Reality spiderX) is an Xray-only hint and has no sing-box
        // outbound field. It is deliberately consumed without copying an
        // unknown key into the generated configuration.
        tls.insert(String::from("reality"), Value::Object(reality_options));
    }
    Ok(Some(Value::Object(tls)))
}

fn server_address(url: &Url, scheme: &str) -> Result<(String, u16), LinkError> {
    let host = url
        .host_str()
        .filter(|host| !host.is_empty())
        .map(str::to_owned)
        .ok_or(LinkError::Invalid)?;
    let default_port = match scheme {
        "vless" | "hysteria2" | "hy2" | "anytls" | "tuic" => 443,
        _ => return Err(LinkError::Unsupported),
    };
    Ok((host, url.port().unwrap_or(default_port)))
}

fn first_query(url: &Url, names: &[&str]) -> Option<String> {
    url.query_pairs().find_map(|(key, value)| {
        names
            .iter()
            .any(|name| key.eq_ignore_ascii_case(name))
            .then(|| value.into_owned())
    })
}

fn query_list(url: &Url, name: &str) -> Vec<String> {
    url.query_pairs()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        .flat_map(|(_, value)| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn userinfo_password(url: &Url) -> Option<String> {
    url.password().and_then(|value| decode_component(value).ok())
}

fn hysteria2_userinfo_password(url: &Url) -> Option<String> {
    let username = decode_component(url.username()).ok()?;
    let password = url
        .password()
        .and_then(|value| decode_component(value).ok());
    match password {
        Some(password) if !username.is_empty() => Some(format!("{username}:{password}")),
        Some(password) => Some(password),
        None => (!username.is_empty()).then_some(username),
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn decode_component(value: &str) -> Result<String, LinkError> {
    percent_decode_str(value)
        .decode_utf8()
        .map(|value| value.into_owned())
        .map_err(|_| LinkError::Invalid)
}

fn decode_body(body: &str) -> String {
    let compact = body.chars().filter(|character| !character.is_whitespace()).collect::<String>();
    if compact.is_empty() {
        return body.to_owned();
    }
    for decoder in [
        general_purpose::STANDARD.decode(&compact),
        general_purpose::STANDARD_NO_PAD.decode(&compact),
        general_purpose::URL_SAFE.decode(&compact),
        general_purpose::URL_SAFE_NO_PAD.decode(&compact),
    ] {
        let Ok(bytes) = decoder else {
            continue;
        };
        let Ok(decoded) = String::from_utf8(bytes) else {
            continue;
        };
        if decoded.lines().any(|line| {
            let line = line.trim_start();
            ["vless://", "hysteria2://", "hy2://", "anytls://", "tuic://"]
                .iter()
                .any(|prefix| line.to_ascii_lowercase().starts_with(prefix))
        }) {
            return decoded;
        }
    }
    body.to_owned()
}

fn fallback_tag(scheme: &str, url: &Url, port: u16) -> String {
    let host = url
        .host_str()
        .unwrap_or("node")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("{scheme}-{host}-{port}")
}

fn clean_tag(value: &str) -> String {
    let cleaned = value
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .trim()
        .to_owned();
    if cleaned.is_empty() {
        String::from("node")
    } else {
        cleaned
    }
}

fn unique_tag(
    base: &str,
    remote: &RemoteSource,
    multi: bool,
    used_tags: &mut HashSet<String>,
) -> String {
    for index in 1.. {
        let candidate = if index == 1 {
            base.to_owned()
        } else {
            format!("{base}-{index}")
        };
        let mut qualified = if multi {
            format!("{}:{candidate}", remote.name)
        } else {
            candidate.clone()
        };
        if RESERVED_TAGS.contains(&candidate.as_str()) {
            qualified = format!("source:{qualified}");
        }
        if used_tags.insert(qualified.clone()) {
            return qualified;
        }
    }
    unreachable!("tag index is unbounded")
}

