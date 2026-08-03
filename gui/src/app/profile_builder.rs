use std::collections::HashSet;

use chrono::{Local, TimeZone};
use rquickjs::{Context, Error, Runtime};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::state::ProfileRemote;

const RESERVED_TAGS: &[&str] = &["PROXY", "direct", "block"];
const HEALTHCHECK_URL: &str = "https://www.gstatic.com/generate_204";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSnapshot {
    pub name: String,
    pub url: String,
    pub proxy_nodes: Vec<Value>,
    pub proxy_groups: Vec<Value>,
    pub updated_at: u64,
}

pub fn parse_remote(
    remote: &ProfileRemote,
    body: &str,
    multi_remote: bool,
) -> Result<RemoteSnapshot, String> {
    let value: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(json_error) => serde_yaml::from_str(body).map_err(|yaml_error| {
            format!(
                "Failed to parse Remote {} (JSON: {json_error}; YAML: {yaml_error})",
                remote.name
            )
        })?,
    };
    let mut snapshot = if value.get("outbounds").is_some() {
        parse_singbox(remote, value, multi_remote)?
    } else {
        parse_clash(remote, value, multi_remote)?
    };
    snapshot.updated_at = u64::try_from(Local::now().timestamp()).unwrap_or_default();
    Ok(snapshot)
}

fn parse_singbox(
    remote: &ProfileRemote,
    value: Value,
    multi_remote: bool,
) -> Result<RemoteSnapshot, String> {
    let outbounds = value
        .get("outbounds")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Remote {} is missing outbounds", remote.name))?;
    let mut proxy_nodes = Vec::new();
    let mut proxy_groups = Vec::new();
    for outbound in outbounds {
        let type_ = outbound
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if matches!(type_, "direct" | "block") {
            continue;
        }
        let normalized = normalize_tag(outbound.clone(), remote, multi_remote, false)?;
        if matches!(type_, "selector" | "urltest" | "fallback" | "loadbalance") {
            proxy_groups.push(normalized);
        } else {
            proxy_nodes.push(normalized);
        }
    }
    Ok(RemoteSnapshot {
        name: remote.name.clone(),
        url: remote.url.clone(),
        proxy_nodes,
        proxy_groups,
        updated_at: 0,
    })
}

fn parse_clash(
    remote: &ProfileRemote,
    value: Value,
    multi_remote: bool,
) -> Result<RemoteSnapshot, String> {
    let proxies = value
        .get("proxies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let groups = value
        .get("proxy-groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut proxy_nodes = Vec::new();
    for proxy in proxies {
        let mut outbound = proxy
            .as_object()
            .cloned()
            .ok_or_else(|| format!("Remote {} contains an invalid Clash proxy", remote.name))?;
        let type_ = outbound
            .remove("type")
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default();
        let type_ = match type_.as_str() {
            "ss" => "shadowsocks",
            "socks5" => "socks",
            value => value,
        };
        if !matches!(
            type_,
            "shadowsocks"
                | "ssr"
                | "vmess"
                | "vless"
                | "trojan"
                | "hysteria2"
                | "tuic"
                | "socks"
                | "http"
        ) {
            log::warn!(
                "skipping unsupported Clash proxy type {type_} ({})",
                remote.name
            );
            continue;
        }
        rename_field(&mut outbound, "port", "server_port");
        rename_field(&mut outbound, "cipher", "method");
        normalize_clash_proxy(&mut outbound, type_);
        outbound.insert(String::from("type"), Value::String(type_.to_string()));
        proxy_nodes.push(normalize_tag(
            Value::Object(outbound),
            remote,
            multi_remote,
            false,
        )?);
    }
    let mut proxy_groups = Vec::new();
    for group in groups {
        let mut outbound = group.as_object().cloned().ok_or_else(|| {
            format!(
                "Remote {} contains an invalid Clash proxy-group",
                remote.name
            )
        })?;
        let type_ = outbound
            .remove("type")
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default();
        let type_ = match type_.as_str() {
            "select" => "selector",
            "url-test" => "urltest",
            "load-balance" | "fallback" => "urltest",
            unsupported => {
                return Err(format!(
                    "Remote {} contains unsupported Clash group type {unsupported}",
                    remote.name
                ));
            }
        };
        let members = outbound
            .remove("proxies")
            .unwrap_or_else(|| Value::Array(Vec::new()));
        let members = normalize_group_members(members);
        outbound.insert(String::from("outbounds"), members);
        outbound.remove("lazy");
        if type_ == "urltest" {
            outbound
                .entry(String::from("url"))
                .or_insert_with(|| Value::String(HEALTHCHECK_URL.into()));
            outbound
                .entry(String::from("interval"))
                .or_insert_with(|| Value::String(String::from("5m")));
            normalize_group_interval(&mut outbound);
        }
        outbound.insert(String::from("type"), Value::String(type_.to_string()));
        proxy_groups.push(normalize_tag(
            Value::Object(outbound),
            remote,
            multi_remote,
            false,
        )?);
    }
    Ok(RemoteSnapshot {
        name: remote.name.clone(),
        url: remote.url.clone(),
        proxy_nodes,
        proxy_groups,
        updated_at: 0,
    })
}

fn rename_field(object: &mut Map<String, Value>, from: &str, to: &str) {
    if let Some(value) = object.remove(from) {
        object.insert(to.into(), value);
    }
}

fn normalize_group_interval(group: &mut Map<String, Value>) {
    if let Some(interval @ Value::Number(_)) = group.get_mut("interval") {
        let seconds = interval.to_string();
        *interval = Value::String(format!("{seconds}s"));
    }
}

fn normalize_group_members(members: Value) -> Value {
    let Some(members) = members.as_array() else {
        return members;
    };
    Value::Array(
        members
            .iter()
            .cloned()
            .map(|member| match member.as_str() {
                Some(name) if name.eq_ignore_ascii_case("DIRECT") => {
                    Value::String(String::from("direct"))
                }
                Some("REJECT" | "REJECT-DROP") => Value::String(String::from("block")),
                _ => member,
            })
            .collect(),
    )
}

fn normalize_clash_proxy(proxy: &mut Map<String, Value>, type_: &str) {
    proxy.remove("alterId");
    proxy.remove("udp");

    let network = proxy
        .remove("network")
        .and_then(|value| value.as_str().map(str::to_string));
    let ws_opts = proxy
        .remove("ws-opts")
        .and_then(|value| value.as_object().cloned());
    let ws_path = proxy.remove("ws-path");
    let ws_headers = proxy.remove("ws-headers");

    if network.as_deref() == Some("ws") {
        let mut transport =
            Map::from_iter([(String::from("type"), Value::String(String::from("ws")))]);
        let path = ws_opts
            .as_ref()
            .and_then(|options| options.get("path"))
            .cloned()
            .or(ws_path);
        let headers = ws_opts
            .as_ref()
            .and_then(|options| options.get("headers"))
            .cloned()
            .or(ws_headers);
        if let Some(path) = path {
            transport.insert(String::from("path"), path);
        }
        if let Some(headers) = headers {
            transport.insert(String::from("headers"), headers);
        }
        proxy.insert(String::from("transport"), Value::Object(transport));
    }

    if let Some(insecure) = proxy.remove("skip-cert-verify") {
        let mut tls = proxy
            .remove("tls")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        tls.insert(String::from("enabled"), Value::Bool(true));
        tls.insert(String::from("insecure"), insecure);
        proxy.insert(String::from("tls"), Value::Object(tls));
    }

    if let Some(server_name) = proxy.remove("sni") {
        let mut tls = proxy
            .remove("tls")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        tls.insert(String::from("enabled"), Value::Bool(true));
        tls.insert(String::from("server_name"), server_name);
        proxy.insert(String::from("tls"), Value::Object(tls));
    }

    if type_ == "vmess" && proxy.get("method") == Some(&Value::String(String::from("auto"))) {
        rename_field(proxy, "method", "security");
    }
}

fn normalize_tag(
    mut outbound: Value,
    remote: &ProfileRemote,
    multi_remote: bool,
    force_source_prefix: bool,
) -> Result<Value, String> {
    let object = outbound
        .as_object_mut()
        .ok_or_else(|| String::from("outbound must be an object"))?;
    let tag = object
        .get("tag")
        .or_else(|| object.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("outbound in Remote {} is missing name", remote.name))?;
    let mut next_tag = tag.to_string();
    if multi_remote {
        next_tag = format!("{}:{next_tag}", remote.name);
    }
    if force_source_prefix || RESERVED_TAGS.contains(&tag) {
        next_tag = format!("source:{next_tag}");
    }
    object.remove("name");
    object.insert(String::from("tag"), Value::String(next_tag));
    Ok(outbound)
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::{normalize_clash_proxy, normalize_group_interval, normalize_group_members};

    #[test]
    fn converts_numeric_clash_group_interval_to_singbox_duration() {
        let mut group = Map::from_iter([(String::from("interval"), json!(600))]);

        normalize_group_interval(&mut group);

        assert_eq!(
            group.get("interval"),
            Some(&Value::String(String::from("600s")))
        );
    }

    #[test]
    fn converts_clash_vmess_transport_and_removes_unsupported_fields() {
        let mut proxy = Map::from_iter([
            (String::from("alterId"), json!(0)),
            (String::from("udp"), json!(true)),
            (String::from("network"), json!("ws")),
            (String::from("ws-path"), json!("/")),
            (String::from("ws-headers"), json!({"Host": "example.com"})),
            (String::from("method"), json!("auto")),
        ]);

        normalize_clash_proxy(&mut proxy, "vmess");

        assert!(!proxy.contains_key("alterId"));
        assert!(!proxy.contains_key("udp"));
        assert!(!proxy.contains_key("network"));
        assert_eq!(proxy.get("security"), Some(&json!("auto")));
        assert_eq!(
            proxy.get("transport"),
            Some(&json!({"type": "ws", "path": "/", "headers": {"Host": "example.com"}}))
        );
    }

    #[test]
    fn maps_clash_builtin_group_members_to_generated_outbounds() {
        assert_eq!(
            normalize_group_members(json!(["DIRECT", "REJECT", "REJECT-DROP", "Proxy"])),
            json!(["direct", "block", "block", "Proxy"])
        );
    }

    #[test]
    fn skips_an_optional_hook_and_preserves_the_configuration() {
        let config = json!({"log": {"level": "info"}});

        let result = run_hook(
            "export function onFinalize(input) { return input.singbox; }",
            "onGenerate",
            json!({"singbox": config}),
        )
        .unwrap();

        assert_eq!(result, json!({"log": {"level": "info"}}));
    }

    #[test]
    fn dispatches_on_generate_with_its_input() {
        let result = run_hook(
            "export function onGenerate(input) {\
                input.singbox.remote_name = input.remote.name; \
                return input.singbox; \
            }",
            "onGenerate",
            json!({"singbox": {}, "remote": {"name": "Primary"}}),
        )
        .unwrap();

        assert_eq!(result, json!({"remote_name": "Primary"}));
    }

    #[test]
    fn rejects_a_hook_return_value_that_is_not_a_configuration_object() {
        let error = run_hook(
            "export function onGenerate() { return []; }",
            "onGenerate",
            json!({"singbox": {}}),
        )
        .unwrap_err();

        assert_eq!(
            error,
            "Profile hook onGenerate must return a configuration object"
        );
    }
}

pub fn build_config(
    mut remotes: Vec<RemoteSnapshot>,
    hook: Option<&str>,
) -> Result<String, String> {
    let multi_remote = remotes.len() > 1;
    let hook_remotes = remotes.clone();
    let mut outbounds = Vec::new();
    let mut groups = Vec::new();
    let mut tags = HashSet::new();
    for remote in &mut remotes {
        dedupe(&mut remote.proxy_nodes, &mut tags);
        dedupe(&mut remote.proxy_groups, &mut tags);
        if multi_remote {
            let mut members = remote
                .proxy_nodes
                .iter()
                .filter_map(tag_of)
                .map(String::from)
                .collect::<Vec<_>>();
            members.push(update_marker(&remote.name, remote.updated_at));
            let marker = members.last().cloned().unwrap_or_default();
            outbounds.push(json!({"type":"direct", "tag": marker}));
            let group = json!({"type":"selector", "tag": remote.name, "outbounds": members});
            groups.push(group.clone());
        }
        outbounds.append(&mut remote.proxy_nodes);
        groups.append(&mut remote.proxy_groups);
    }
    let group_tags = groups
        .iter()
        .filter_map(tag_of)
        .map(String::from)
        .collect::<Vec<_>>();
    let node_tags = outbounds
        .iter()
        .filter_map(tag_of)
        .filter(|tag| !group_tags.iter().any(|group| group == tag))
        .map(String::from)
        .collect::<Vec<_>>();
    outbounds.extend(groups);
    outbounds.push(json!({"type":"selector", "tag":"PROXY", "outbounds": if group_tags.is_empty() { node_tags } else { group_tags }}));
    outbounds.push(json!({"type":"direct", "tag":"direct"}));
    outbounds.push(json!({"type":"block", "tag":"block"}));
    let mut config = template(outbounds);
    if let Some(hook) = hook.filter(|value| !value.trim().is_empty()) {
        let mut input = json!({"singbox": config});
        if multi_remote {
            input["remotes"] = serde_json::to_value(hook_remotes).map_err(|err| err.to_string())?;
        } else if let Some(remote) = hook_remotes.into_iter().next() {
            input["remote"] = serde_json::to_value(remote).map_err(|err| err.to_string())?;
        }
        config = run_hook(hook, "onGenerate", input)?;
    }
    serde_json::to_string_pretty(&config)
        .map_err(|err| format!("Failed to serialize generated configuration: {err}"))
}

fn dedupe(items: &mut Vec<Value>, seen: &mut HashSet<String>) {
    items.retain(|item| tag_of(item).is_some_and(|tag| seen.insert(tag.to_string())));
}
fn tag_of(value: &Value) -> Option<&str> {
    value.get("tag").and_then(Value::as_str)
}
fn update_marker(name: &str, timestamp: u64) -> String {
    let time = Local
        .timestamp_opt(timestamp as i64, 0)
        .single()
        .unwrap_or_else(Local::now);
    format!("{name}:({})", time.format("%Y/%m/%d %H:%M"))
}

fn template(outbounds: Vec<Value>) -> Value {
    // GLOBAL is intentionally not declared: the product requirement expects sing-box to provide it.
    json!({"outbounds": outbounds, "route": {"final":"direct", "rules":[{"action":"hijack-dns","protocol":"dns"},{"action":"route","outbound":"direct","clash_mode":"direct"},{"action":"route","outbound":"GLOBAL","clash_mode":"global"},{"action":"route","outbound":"direct","network":"icmp"},{"action":"route","outbound":"block","protocol":"quic"},{"action":"route","outbound":"block","rule_set":["Category-Ads"]},{"action":"route","outbound":"direct","rule_set":["GeoSite-Private","GeoSite-CN","GeoIP-Private","GeoIP-CN"]},{"action":"route","outbound":"PROXY","rule_set":["GeoLocation-!CN"]}],"rule_set":[{"tag":"Category-Ads","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/category-ads-all.srs","format":"binary","download_detour":"direct"},{"tag":"GeoIP-Private","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/private.srs","format":"binary","download_detour":"direct"},{"tag":"GeoSite-Private","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/private.srs","format":"binary","download_detour":"direct"},{"tag":"GeoIP-CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/cn.srs","format":"binary","download_detour":"direct"},{"tag":"GeoSite-CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/cn.srs","format":"binary","download_detour":"direct"},{"tag":"GeoLocation-!CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/geolocation-!cn.srs","format":"binary","download_detour":"direct"}],"auto_detect_interface":true,"default_domain_resolver":{"server":"Local-DNS"}},"dns":{"final":"Remote-DNS","servers":[{"tag":"Fake-IP","type":"fakeip","inet4_range":"198.18.0.0/15","inet6_range":"fc00::/18"},{"tag":"Local-DNS","type":"https","server":"223.5.5.5","server_port":443,"path":"/dns-query","domain_resolver":"Local-DNS-Resolver"},{"tag":"Local-DNS-Resolver","type":"udp","server":"223.5.5.5","server_port":53},{"tag":"Remote-DNS","type":"tls","server":"8.8.8.8","server_port":853,"detour":"PROXY","domain_resolver":"Remote-DNS-Resolver"},{"tag":"Remote-DNS-Resolver","type":"udp","server":"8.8.8.8","server_port":53,"detour":"PROXY"},{"tag":"dns-device-local","type":"local"}],"rules":[{"action":"route","server":"Local-DNS","clash_mode":"direct"},{"action":"route","server":"Remote-DNS","clash_mode":"global"},{"action":"route","server":"Local-DNS","rule_set":["GeoSite-CN"]},{"action":"route","server":"Remote-DNS","rule_set":["GeoLocation-!CN"]}],"independent_cache":false,"disable_cache":false,"disable_expire":false}})
}

pub fn run_finalize_hook(source: &str, config: Value) -> Result<Value, String> {
    run_hook(source, "onFinalize", json!({"singbox": config}))
}

fn run_hook(source: &str, function_name: &str, input: Value) -> Result<Value, String> {
    let runtime = Runtime::new()
        .map_err(|err| format!("Failed to initialize Profile hook runtime: {err}"))?;
    let context = Context::full(&runtime)
        .map_err(|err| format!("Failed to initialize Profile hook context: {err}"))?;
    let input = serde_json::to_string(&input).map_err(|err| err.to_string())?;
    let script = format!(
        "{}\ntypeof {function_name} === 'function' ? JSON.stringify({function_name}({input})) : JSON.stringify(({input}).singbox)",
        source.replace("export ", "")
    );
    context
        .with(|ctx| match ctx.eval::<Option<String>, _>(script) {
            Ok(value) => Ok(value),
            Err(error) => {
                if matches!(&error, Error::Exception) {
                    let exception = ctx.catch();
                    if let Some(exception) = exception.as_exception() {
                        let message = exception.message().unwrap_or_default();
                        let stack = exception.stack().unwrap_or_default();
                        log::error!(
                            "Profile hook QuickJS exception: message={} stack={}",
                            message,
                            stack
                        );
                        return Err(format!(
                            "Profile hook {function_name} execution failed:\nmessage: {message}\nstack:\n{stack}"
                        ));
                    } else {
                        log::error!(
                            "Profile hook QuickJS exception: unable to read exception object"
                        );
                    }
                } else {
                    log::error!("Profile hook execution failed: {error}");
                }
                Err(format!("Profile hook {function_name} execution failed: {error}"))
            }
        })
        .and_then(|value| {
            let value = value.ok_or_else(|| {
                format!("Profile hook {function_name} did not return a configuration object")
            })?;
            let value: Value = serde_json::from_str(&value).map_err(|err| {
                format!("Profile hook {function_name} returned invalid JSON: {err}")
            })?;
            if !value.is_object() {
                return Err(format!(
                    "Profile hook {function_name} must return a configuration object"
                ));
            }
            Ok(value)
        })
}
