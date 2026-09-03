use std::collections::HashSet;

use serde_json::{Value, json};

use super::{RemoteSnapshot, hooks::run_generate_hook};

/// Assembles a profile from its sing-box template and ordered Remote snapshots.
/// Template arrays remain first; source arrays are appended in remote order.
pub fn build_config(
    template: &str,
    mut remotes: Vec<RemoteSnapshot>,
    hook: Option<&str>,
) -> Result<String, String> {
    let mut config: Value = serde_json::from_str(template)
        .map_err(|error| format!("Failed to parse Template sing-box configuration: {error}"))?;
    if !config.is_object() {
        return Err(String::from(
            "Template sing-box configuration must be a JSON object.",
        ));
    }
    let multi = remotes.len() > 1;
    let hook_remotes = remotes.clone();
    let mut outbounds = config
        .get("outbounds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut route_rules = config
        .pointer("/route/rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut source_groups = Vec::new();
    let mut source_nodes = Vec::new();
    for remote in &mut remotes {
        source_nodes.append(&mut remote.proxy_nodes);
        source_groups.append(&mut remote.proxy_groups);
        route_rules.append(&mut remote.route_rules);
        merge_preserved_fields(&mut config, remote);
        if let Some(final_outbound) = remote.route_final.take() {
            config["route"]["final"] = Value::String(final_outbound);
        }
    }
    let group_tags: Vec<String> = source_groups
        .iter()
        .filter_map(tag)
        .map(str::to_owned)
        .collect();
    let node_tags: Vec<String> = source_nodes
        .iter()
        .filter(|outbound| !is_builtin_outbound(outbound))
        .filter_map(tag)
        .map(str::to_owned)
        .collect();
    outbounds.append(&mut source_nodes);
    outbounds.append(&mut source_groups);
    let mut members = group_tags;
    members.extend(node_tags);
    if members.is_empty() {
        members.push(String::from("direct"));
    }
    match outbounds.iter_mut().find(|item| tag(item) == Some("PROXY")) {
        Some(proxy) => append_proxy_members(proxy, &members)?,
        None => outbounds.push(json!({"type":"selector","tag":"PROXY","outbounds":members})),
    }
    dedupe_outbounds(&mut outbounds);
    config["outbounds"] = Value::Array(outbounds);
    if !route_rules.is_empty() {
        config["route"]["rules"] = Value::Array(route_rules);
    }
    if let Some(source) = hook.filter(|source| !source.trim().is_empty()) {
        let mut input = json!({"singbox": config});
        if multi {
            input["remotes"] =
                serde_json::to_value(hook_remotes).map_err(|error| error.to_string())?;
        } else if let Some(remote) = hook_remotes.into_iter().next() {
            input["remote"] = serde_json::to_value(remote).map_err(|error| error.to_string())?;
        }
        config = run_generate_hook(source, input)?;
    }
    serde_json::to_string_pretty(&config)
        .map_err(|error| format!("Failed to serialize generated configuration: {error}"))
}

fn merge_preserved_fields(config: &mut Value, remote: &mut RemoteSnapshot) {
    let mut fragment = serde_json::Map::new();
    if let Some(dns) = remote.dns.take() {
        fragment.insert(String::from("dns"), dns);
    }
    if !remote.inbounds.is_empty() {
        fragment.insert(
            String::from("inbounds"),
            Value::Array(std::mem::take(&mut remote.inbounds)),
        );
    }
    if let Some(route) = remote.route.take() {
        fragment.insert(String::from("route"), route);
    }
    if let Some(experimental) = remote.experimental.take() {
        fragment.insert(String::from("experimental"), experimental);
    }
    merge_value(config, Value::Object(fragment));
}

fn merge_value(target: &mut Value, incoming: Value) {
    match (target, incoming) {
        (Value::Object(target), Value::Object(incoming)) => {
            for (key, value) in incoming {
                match target.get_mut(&key) {
                    Some(target) => merge_value(target, value),
                    None => {
                        target.insert(key, value);
                    }
                }
            }
        }
        (Value::Array(target), Value::Array(mut incoming)) => {
            // TODO: Define field-specific deduplication rules before removing duplicates here.
            target.append(&mut incoming);
        }
        (target, incoming) => *target = incoming,
    }
}

fn append_proxy_members(proxy: &mut Value, members: &[String]) -> Result<(), String> {
    let outbounds = proxy
        .as_object_mut()
        .and_then(|object| object.get_mut("outbounds"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| String::from("Template PROXY outbound must contain an outbounds array."))?;
    for member in members {
        if !outbounds.iter().any(|item| item.as_str() == Some(member)) {
            outbounds.push(Value::String(member.clone()));
        }
    }
    Ok(())
}
fn dedupe_outbounds(outbounds: &mut Vec<Value>) {
    let mut seen_tags = HashSet::new();
    let mut untagged = Vec::new();
    let mut retained = Vec::with_capacity(outbounds.len());
    for outbound in std::mem::take(outbounds).into_iter().rev() {
        let keep = match tag(&outbound) {
            Some(tag) => seen_tags.insert(tag.to_owned()),
            None => {
                if untagged.contains(&outbound) {
                    false
                } else {
                    untagged.push(outbound.clone());
                    true
                }
            }
        };
        if keep {
            retained.push(outbound);
        }
    }
    retained.reverse();
    *outbounds = retained;
}

fn is_builtin_outbound(outbound: &Value) -> bool {
    matches!(
        outbound.get("type").and_then(Value::as_str),
        Some("direct" | "block")
    )
}
fn tag(value: &Value) -> Option<&str> {
    value.get("tag").and_then(Value::as_str)
}

/// The profile editor's quick-create template. All policy lives in this JSON.
pub fn default_template() -> Value {
    json!({
        "inbounds":[{"type":"mixed","tag":"mixed-in","listen":"127.0.0.1","listen_port":7890}],
        "outbounds":[{"type":"direct","tag":"direct"},{"type":"block","tag":"block"}],
        "route":{"final":"direct","auto_detect_interface":true,"default_domain_resolver":{"server":"Local-DNS"},
          "rule_set":[
            {"tag":"Category-Ads","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/category-ads-all.srs","format":"binary"},
            {"tag":"GeoIP-Private","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/private.srs","format":"binary"},
            {"tag":"GeoSite-Private","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/private.srs","format":"binary"},
            {"tag":"GeoIP-CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/cn.srs","format":"binary"},
            {"tag":"GeoSite-CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/cn.srs","format":"binary"},
            {"tag":"GeoLocation-!CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/geolocation-!cn.srs","format":"binary"}],
          "rules":[{"action":"hijack-dns","protocol":"dns"},{"action":"route","outbound":"direct","clash_mode":"direct"},{"action":"route","outbound":"direct","network":"icmp"},{"action":"route","outbound":"block","protocol":"quic"},{"action":"route","outbound":"block","rule_set":["Category-Ads"]},{"action":"route","outbound":"direct","rule_set":["GeoSite-Private","GeoSite-CN","GeoIP-Private","GeoIP-CN"]},{"action":"route","outbound":"PROXY","rule_set":["GeoLocation-!CN"]}]},
        "dns":{"final":"Remote-DNS","servers":[{"tag":"Fake-IP","type":"fakeip","inet4_range":"198.18.0.0/15","inet6_range":"fc00::/18"},{"tag":"Local-DNS","type":"https","server":"223.5.5.5","server_port":443,"path":"/dns-query","domain_resolver":"Local-DNS-Resolver"},{"tag":"Local-DNS-Resolver","type":"udp","server":"223.5.5.5","server_port":53},{"tag":"Remote-DNS","type":"tls","server":"8.8.8.8","server_port":853,"detour":"PROXY","domain_resolver":"Remote-DNS-Resolver"},{"tag":"Remote-DNS-Resolver","type":"udp","server":"8.8.8.8","server_port":53,"detour":"PROXY"},{"tag":"dns-device-local","type":"local"}],"rules":[{"action":"route","server":"Local-DNS","clash_mode":"direct"},{"action":"route","server":"Remote-DNS","clash_mode":"global"},{"action":"route","server":"Local-DNS","rule_set":["GeoSite-CN"]},{"action":"route","server":"Remote-DNS","rule_set":["GeoLocation-!CN"]}],"independent_cache":false,"disable_cache":false,"disable_expire":false}
    })
}
