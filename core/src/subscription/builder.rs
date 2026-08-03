use std::collections::HashSet;

use chrono::{Local, TimeZone};
use serde_json::{Value, json};

use super::{RemoteSnapshot, hooks::run_generate_hook};

pub fn build_config(
    mut remotes: Vec<RemoteSnapshot>,
    hook: Option<&str>,
) -> Result<String, String> {
    let multi = remotes.len() > 1;
    let hook_remotes = remotes.clone();
    let mut nodes = Vec::new();
    let mut groups = Vec::new();
    let mut rules = Vec::new();
    let mut seen = HashSet::new();
    for remote in &mut remotes {
        dedupe(&mut remote.proxy_nodes, &mut seen);
        dedupe(&mut remote.proxy_groups, &mut seen);
        if multi {
            let mut members: Vec<_> = remote
                .proxy_nodes
                .iter()
                .filter_map(tag)
                .map(str::to_owned)
                .collect();
            let marker = format!(
                "{}:({})",
                remote.name,
                Local
                    .timestamp_opt(remote.updated_at as i64, 0)
                    .single()
                    .unwrap_or_else(Local::now)
                    .format("%Y/%m/%d %H:%M")
            );
            members.push(marker.clone());
            nodes.push(json!({"type":"direct","tag":marker}));
            groups.push(json!({"type":"selector","tag":remote.name,"outbounds":members}));
        }
        nodes.append(&mut remote.proxy_nodes);
        groups.append(&mut remote.proxy_groups);
        rules.append(&mut remote.route_rules);
    }
    let group_tags: Vec<_> = groups.iter().filter_map(tag).map(str::to_owned).collect();
    let node_tags: Vec<_> = nodes
        .iter()
        .filter_map(tag)
        .filter(|tag| !group_tags.iter().any(|group| group == tag))
        .map(str::to_owned)
        .collect();
    nodes.extend(groups);
    nodes.push(json!({"type":"selector","tag":"PROXY","outbounds":if group_tags.is_empty(){node_tags}else{group_tags}}));
    nodes.push(json!({"type":"direct","tag":"direct"}));
    nodes.push(json!({"type":"block","tag":"block"}));
    let mut config = template(nodes, merge_route_rules(rules));
    if let Some(source) = hook.filter(|source| !source.trim().is_empty()) {
        let mut input = json!({"singbox":config});
        if multi {
            input["remotes"] = serde_json::to_value(hook_remotes).map_err(|e| e.to_string())?;
        } else if let Some(remote) = hook_remotes.into_iter().next() {
            input["remote"] = serde_json::to_value(remote).map_err(|e| e.to_string())?;
        }
        config = run_generate_hook(source, input)?;
    }
    serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize generated configuration: {e}"))
}

fn template(outbounds: Vec<Value>, mut rules: Vec<Value>) -> Value {
    rules.extend([json!({"action":"hijack-dns","protocol":"dns"}), json!({"action":"route","outbound":"direct","clash_mode":"direct"}), json!({"action":"route","outbound":"GLOBAL","clash_mode":"global"}), json!({"action":"route","outbound":"direct","network":"icmp"}), json!({"action":"route","outbound":"block","protocol":"quic"}), json!({"action":"route","outbound":"block","rule_set":["Category-Ads"]}), json!({"action":"route","outbound":"direct","rule_set":["GeoSite-Private","GeoSite-CN","GeoIP-Private","GeoIP-CN"]}), json!({"action":"route","outbound":"PROXY","rule_set":["GeoLocation-!CN"]})]);
    json!({"outbounds":outbounds,"route":{"final":"direct","rules":rules,"rule_set":[{"tag":"Category-Ads","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/category-ads-all.srs","format":"binary","download_detour":"direct"},{"tag":"GeoIP-Private","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/private.srs","format":"binary","download_detour":"direct"},{"tag":"GeoSite-Private","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/private.srs","format":"binary","download_detour":"direct"},{"tag":"GeoIP-CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/cn.srs","format":"binary","download_detour":"direct"},{"tag":"GeoSite-CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/cn.srs","format":"binary","download_detour":"direct"},{"tag":"GeoLocation-!CN","type":"remote","url":"https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/geolocation-!cn.srs","format":"binary","download_detour":"direct"}],"auto_detect_interface":true,"default_domain_resolver":{"server":"Local-DNS"}},"dns":{"final":"Remote-DNS","servers":[{"tag":"Fake-IP","type":"fakeip","inet4_range":"198.18.0.0/15","inet6_range":"fc00::/18"},{"tag":"Local-DNS","type":"https","server":"223.5.5.5","server_port":443,"path":"/dns-query","domain_resolver":"Local-DNS-Resolver"},{"tag":"Local-DNS-Resolver","type":"udp","server":"223.5.5.5","server_port":53},{"tag":"Remote-DNS","type":"tls","server":"8.8.8.8","server_port":853,"detour":"PROXY","domain_resolver":"Remote-DNS-Resolver"},{"tag":"Remote-DNS-Resolver","type":"udp","server":"8.8.8.8","server_port":53,"detour":"PROXY"},{"tag":"dns-device-local","type":"local"}],"rules":[{"action":"route","server":"Local-DNS","clash_mode":"direct"},{"action":"route","server":"Remote-DNS","clash_mode":"global"},{"action":"route","server":"Local-DNS","rule_set":["GeoSite-CN"]},{"action":"route","server":"Remote-DNS","rule_set":["GeoLocation-!CN"]}],"independent_cache":false,"disable_cache":false,"disable_expire":false}})
}

const ROUTE_MATCHERS: &[&str] = &[
    "domain",
    "domain_suffix",
    "domain_keyword",
    "ip_cidr",
    "process_name",
    "port",
];

/// Merges consecutive source rules only. Route rules are evaluated in order, so
/// merging across an intervening rule could change which action handles a match.
fn merge_route_rules(rules: Vec<Value>) -> Vec<Value> {
    let mut merged = Vec::with_capacity(rules.len());
    for rule in rules {
        if let Some(previous) = merged.last_mut() {
            let matcher = simple_route_matcher(previous).map(|(matcher, _)| matcher);
            let values = simple_route_matcher(&rule).map(|(matcher, values)| (matcher, values));
            if let (Some(matcher), Some((rule_matcher, values))) = (matcher, values)
                && matcher == rule_matcher
                && compatible_route_rules(previous, &rule, matcher)
            {
                let previous_values = previous
                    .get_mut(matcher)
                    .and_then(Value::as_array_mut)
                    .expect("simple route matcher must remain an array");
                for value in values {
                    if !previous_values.contains(value) {
                        previous_values.push(value.clone());
                    }
                }
                continue;
            }
        }
        merged.push(rule);
    }
    merged
}

fn simple_route_matcher(rule: &Value) -> Option<(&'static str, &Vec<Value>)> {
    let object = rule.as_object()?;
    let mut matchers = ROUTE_MATCHERS.iter().filter_map(|key| {
        object
            .get(*key)
            .and_then(Value::as_array)
            .map(|values| (*key, values))
    });
    let matcher = matchers.next()?;
    if matchers.next().is_some() {
        return None;
    }
    Some(matcher)
}

fn compatible_route_rules(left: &Value, right: &Value, matcher: &str) -> bool {
    let (Some(left), Some(right)) = (left.as_object(), right.as_object()) else {
        return false;
    };
    let mut left_without_matcher = left.clone();
    let mut right_without_matcher = right.clone();
    left_without_matcher.remove(matcher);
    right_without_matcher.remove(matcher);
    matches!(left.get("action"), Some(Value::String(_)))
        && left.get("action") == right.get("action")
        && matches!(left.get("outbound"), Some(Value::String(_)))
        && left.get("outbound") == right.get("outbound")
        && left_without_matcher == right_without_matcher
}

fn dedupe(items: &mut Vec<Value>, seen: &mut HashSet<String>) {
    items.retain(|item| tag(item).is_some_and(|tag| seen.insert(tag.into())));
}

fn tag(value: &Value) -> Option<&str> {
    value.get("tag").and_then(Value::as_str)
}
