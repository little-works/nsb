use std::collections::HashSet;
use std::str::FromStr;

use chrono::Local;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::config::AppConfig;
use crate::state::{
    ProfileHeader, ProfileItem, ProfileRemote, ProfileTemplate, current_timestamp,
    generate_profile_id,
};

pub const PORTABLE_DATA_FORMAT: &str = "nsb-portable-data";
pub const PORTABLE_DATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PortableTemplate {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PortableProfile {
    pub name: String,
    pub template_name: Option<String>,
    pub inline_template: Option<String>,
    pub remotes: Vec<ProfileRemote>,
    pub hook: Option<String>,
    pub update_interval_hours: Option<u32>,
    pub update_cron: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PortableDataArchive {
    pub format: String,
    pub version: u32,
    pub templates: Vec<PortableTemplate>,
    pub profiles: Vec<PortableProfile>,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum DataImportIssueLevel {
    Warning,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum DataImportEntityKind {
    Template,
    Profile,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DataImportIssue {
    pub level: DataImportIssueLevel,
    pub entity: DataImportEntityKind,
    pub name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct DataImportEntityStats {
    pub added: usize,
    pub overwritten: usize,
    pub skipped: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct DataImportReport {
    pub templates: DataImportEntityStats,
    pub profiles: DataImportEntityStats,
    pub issues: Vec<DataImportIssue>,
}

impl DataImportReport {
    pub fn actionable_count(&self) -> usize {
        self.templates.added
            + self.templates.overwritten
            + self.profiles.added
            + self.profiles.overwritten
    }
}

#[derive(Debug)]
pub struct DataImportPlan {
    pub config: AppConfig,
    pub report: DataImportReport,
    pub imported_profile_ids: Vec<String>,
}

#[derive(Deserialize)]
struct RawArchive {
    format: String,
    version: u32,
    templates: Vec<Value>,
    profiles: Vec<Value>,
}

pub fn export_archive(config: &AppConfig) -> PortableDataArchive {
    PortableDataArchive {
        format: PORTABLE_DATA_FORMAT.to_string(),
        version: PORTABLE_DATA_VERSION,
        templates: config
            .templates
            .iter()
            .map(|item| PortableTemplate {
                name: item.name.clone(),
                content: item.content.clone(),
            })
            .collect(),
        profiles: config
            .profiles
            .iter()
            .map(|item| PortableProfile {
                name: item.name.clone(),
                template_name: config
                    .templates
                    .iter()
                    .find(|template| template.id == item.template_id)
                    .map(|template| template.name.clone()),
                inline_template: item.inline_template.clone(),
                remotes: item.remotes.clone(),
                hook: item.hook.clone(),
                update_interval_hours: item.update_interval_hours,
                update_cron: item.update_cron.clone(),
            })
            .collect(),
    }
}

pub fn plan_import(content: &str, current: &AppConfig) -> Result<DataImportPlan, String> {
    if content.trim().is_empty() {
        return Err(String::from("Imported data file is empty."));
    }
    let raw: RawArchive = serde_json::from_str(content)
        .map_err(|error| format!("Failed to parse imported data JSON: {error}"))?;
    if raw.format != PORTABLE_DATA_FORMAT {
        return Err(String::from("The selected file is not an NSB portable data backup."));
    }
    if raw.version != PORTABLE_DATA_VERSION {
        return Err(format!(
            "Unsupported portable data version: {}.",
            raw.version
        ));
    }

    let mut report = DataImportReport::default();
    let templates = parse_templates(raw.templates, &mut report);
    let profiles = parse_profiles(raw.profiles, &mut report);
    let mut config = current.clone();
    let now = current_timestamp();

    for template in templates {
        let existing_index = config
            .templates
            .iter()
            .position(|existing| existing.name == template.name);
        let id = existing_index
            .map(|index| config.templates[index].id.clone())
            .unwrap_or_else(|| next_template_id(&config));
        let item = ProfileTemplate {
            id,
            name: template.name,
            content: template.content,
            updated_at: now,
            reference_count: 0,
        };
        if let Some(index) = existing_index {
            config.templates[index] = item;
            report.templates.overwritten += 1;
        } else {
            config.templates.push(item);
            report.templates.added += 1;
        }
    }

    let available_templates = config
        .templates
        .iter()
        .map(|template| (template.name.clone(), template.id.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut imported_profile_ids = Vec::new();
    for profile in profiles {
        let existing_index = config
            .profiles
            .iter()
            .position(|existing| existing.name == profile.name);
        let id = existing_index
            .map(|index| config.profiles[index].id.clone())
            .unwrap_or_else(|| next_profile_id(&config));
        let revision = existing_index
            .map(|index| config.profiles[index].revision.saturating_add(1))
            .unwrap_or(1);
        let next_update_at = next_update_at(
            profile.update_interval_hours,
            profile.update_cron.as_deref(),
            now,
        );
        let template_id = match profile.template_name.as_deref() {
            Some(name) => match available_templates.get(name) {
                Some(id) => id.clone(),
                None => {
                    report.profiles.skipped += 1;
                    report.issues.push(DataImportIssue {
                        level: DataImportIssueLevel::Skipped,
                        entity: DataImportEntityKind::Profile,
                        name: Some(profile.name),
                        reason: format!("Referenced Template '{name}' does not exist."),
                    });
                    continue;
                }
            },
            None => String::new(),
        };
        let item = ProfileItem {
            id: id.clone(),
            name: profile.name.clone(),
            template_id,
            inline_template: profile.inline_template,
            updated_at: now,
            remotes: profile.remotes,
            hook: profile.hook,
            update_interval_hours: profile.update_interval_hours,
            update_cron: profile.update_cron,
            next_update_at,
            last_attempt_at: 0,
            last_update_error: None,
            revision,
        };

        if current.current_profile_id.as_deref() == Some(item.id.as_str()) {
            report.profiles.warnings += 1;
            report.issues.push(DataImportIssue {
                level: DataImportIssueLevel::Warning,
                entity: DataImportEntityKind::Profile,
                name: Some(item.name.clone()),
                reason: String::from(
                    "This is the current Profile. Refresh it manually to apply the imported definition.",
                ),
            });
        }

        if let Some(index) = existing_index {
            config.profiles[index] = item;
            report.profiles.overwritten += 1;
        } else {
            config.profiles.push(item);
            report.profiles.added += 1;
        }
        imported_profile_ids.push(id);
    }

    Ok(DataImportPlan {
        config,
        report,
        imported_profile_ids,
    })
}

fn parse_templates(values: Vec<Value>, report: &mut DataImportReport) -> Vec<PortableTemplate> {
    parse_last_wins(
        values,
        DataImportEntityKind::Template,
        &mut report.templates,
        &mut report.issues,
        |value| {
            let mut item: PortableTemplate = serde_json::from_value(value)
                .map_err(|error| format!("Invalid Template record: {error}"))?;
            item.name = item.name.trim().to_string();
            if item.name.is_empty() {
                return Err(String::from("Template name cannot be empty."));
            }
            let parsed: Value = serde_json::from_str(&item.content)
                .map_err(|error| format!("Template content is not valid JSON: {error}"))?;
            if !parsed.is_object() {
                return Err(String::from("Template content must be a JSON object."));
            }
            serde_json::from_value::<nsb_core::SingBoxConfig>(parsed.clone())
                .map_err(|error| format!("Template content is not valid sing-box JSON: {error}"))?;
            item.content = serde_json::to_string_pretty(&parsed)
                .map_err(|error| format!("Failed to normalize Template content: {error}"))?;
            Ok(item)
        },
    )
}

fn parse_profiles(values: Vec<Value>, report: &mut DataImportReport) -> Vec<PortableProfile> {
    parse_last_wins(
        values,
        DataImportEntityKind::Profile,
        &mut report.profiles,
        &mut report.issues,
        |value| {
            let mut item: PortableProfile = serde_json::from_value(value)
                .map_err(|error| format!("Invalid Profile record: {error}"))?;
            item.name = item.name.trim().to_string();
            if item.name.is_empty() {
                return Err(String::from("Profile name cannot be empty."));
            }
            validate_schedule(item.update_interval_hours, item.update_cron.as_deref())?;
            for remote in &item.remotes {
                validate_headers(&remote.headers)?;
            }
            validate_remote_names(&item.remotes)?;

            item.template_name = item
                .template_name
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty());
            item.inline_template = item
                .inline_template
                .map(|content| content.trim().to_string())
                .filter(|content| !content.is_empty());
            match (item.template_name.as_deref(), item.inline_template.as_deref()) {
                (None, Some(content)) => {
                    serde_json::from_str::<nsb_core::SingBoxConfig>(content).map_err(|error| {
                        format!("Inline Template is not valid sing-box JSON: {error}")
                    })?;
                }
                (Some(_), None) => {}
                (None, None) => return Err(String::from("A Template is required.")),
                (Some(_), Some(_)) => {
                    return Err(String::from(
                        "Choose either an inline Template or a shared Template.",
                    ));
                }
            }
            item.hook = item
                .hook
                .map(|hook| hook.trim().to_string())
                .filter(|hook| !hook.is_empty());
            Ok(item)
        },
    )
}

fn parse_last_wins<T>(
    values: Vec<Value>,
    entity: DataImportEntityKind,
    stats: &mut DataImportEntityStats,
    issues: &mut Vec<DataImportIssue>,
    validate: impl Fn(Value) -> Result<T, String>,
) -> Vec<T> {
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    for value in values.into_iter().rev() {
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .map(ToOwned::to_owned);
        if name
            .as_ref()
            .is_some_and(|name| !seen.insert(name.clone()))
        {
            continue;
        }
        selected.push((name, value));
    }
    selected.reverse();

    selected
        .into_iter()
        .filter_map(|(name, value)| {
            match validate(value) {
                Ok(item) => Some(item),
                Err(reason) => {
                    stats.skipped += 1;
                    issues.push(DataImportIssue {
                        level: DataImportIssueLevel::Skipped,
                        entity,
                        name,
                        reason,
                    });
                    None
                }
            }
        })
        .collect()
}

fn next_template_id(config: &AppConfig) -> String {
    loop {
        let id = generate_profile_id();
        if !config.templates.iter().any(|item| item.id == id) {
            return id;
        }
    }
}

fn next_profile_id(config: &AppConfig) -> String {
    loop {
        let id = generate_profile_id();
        if !config.profiles.iter().any(|item| item.id == id) {
            return id;
        }
    }
}

fn validate_schedule(interval: Option<u32>, cron_expression: Option<&str>) -> Result<(), String> {
    if interval == Some(0) {
        return Err(String::from("Update interval must be greater than 0."));
    }
    if interval.is_some() && cron_expression.is_some_and(|value| !value.trim().is_empty()) {
        return Err(String::from(
            "Only one of update interval and Cron can be set.",
        ));
    }
    if let Some(value) = cron_expression.map(str::trim).filter(|value| !value.is_empty()) {
        Schedule::from_str(&format!("0 {value} *"))
            .map_err(|error| format!("Invalid Cron expression: {error}"))?;
    }
    Ok(())
}

fn validate_headers(headers: &[ProfileHeader]) -> Result<(), String> {
    for header in headers {
        if header.key.trim().is_empty() {
            return Err(String::from("Profile Header name cannot be empty."));
        }
        reqwest::header::HeaderName::from_bytes(header.key.trim().as_bytes())
            .map_err(|error| format!("Invalid Profile Header name: {error}"))?;
        reqwest::header::HeaderValue::from_str(&header.value)
            .map_err(|error| format!("Invalid Profile Header value: {error}"))?;
    }
    Ok(())
}

fn validate_remote_names(remotes: &[ProfileRemote]) -> Result<(), String> {
    if remotes.len() <= 1 {
        return Ok(());
    }
    let mut names = HashSet::new();
    for remote in remotes {
        let name = remote.name.trim();
        if name.is_empty()
            || !names.insert(name.to_string())
            || matches!(name, "PROXY" | "direct" | "block")
        {
            return Err(String::from(
                "Remote names must be unique and cannot be PROXY, direct, or block.",
            ));
        }
    }
    Ok(())
}

fn next_update_at(interval: Option<u32>, cron_expression: Option<&str>, now: u64) -> u64 {
    if let Some(hours) = interval {
        return now.saturating_add(u64::from(hours).saturating_mul(60 * 60));
    }
    let Some(value) = cron_expression.map(str::trim).filter(|value| !value.is_empty()) else {
        return 0;
    };
    Schedule::from_str(&format!("0 {value} *"))
        .ok()
        .and_then(|schedule| schedule.after(&Local::now()).next())
        .and_then(|time| u64::try_from(time.timestamp()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::config::AppConfig;

    fn template_content() -> String {
        serde_json::to_string(&nsb_core::default_template()).expect("default Template serializes")
    }

    fn profile(id: &str, name: &str, template_id: &str) -> ProfileItem {
        ProfileItem {
            id: id.to_string(),
            name: name.to_string(),
            template_id: template_id.to_string(),
            inline_template: None,
            updated_at: 12,
            remotes: Vec::new(),
            hook: None,
            update_interval_hours: Some(24),
            update_cron: None,
            next_update_at: 34,
            last_attempt_at: 56,
            last_update_error: Some(String::from("old failure")),
            revision: 7,
        }
    }

    #[test]
    fn export_contains_only_portable_template_and_profile_fields() {
        let mut config = AppConfig::default();
        config.templates.push(ProfileTemplate {
            id: String::from("template-one"),
            name: String::from("Default"),
            content: template_content(),
            updated_at: 100,
            reference_count: 3,
        });
        let mut item = profile("profile-one", "Main", "template-one");
        item.hook = Some(String::from("export function onFinalize(input) { return input.singbox }"));
        config.profiles.push(item);
        config.current_profile_id = Some(String::from("profile-one"));

        let value = serde_json::to_value(export_archive(&config)).expect("archive serializes");
        assert!(value["templates"][0].get("id").is_none());
        assert!(value["profiles"][0].get("id").is_none());
        assert_eq!(value["profiles"][0]["template_name"], "Default");
        assert!(value["profiles"][0]["hook"].as_str().is_some());
        for field in [
            "current_profile_id",
            "updated_at",
            "next_update_at",
            "last_attempt_at",
            "last_update_error",
            "revision",
            "reference_count",
        ] {
            assert!(!value.to_string().contains(field));
        }
    }

    #[test]
    fn last_invalid_duplicate_name_keeps_local_item() {
        let mut config = AppConfig::default();
        config.templates.push(ProfileTemplate {
            id: String::from("template-one"),
            name: String::from("Local"),
            content: template_content(),
            updated_at: 1,
            reference_count: 0,
        });
        let content = json!({
            "format": PORTABLE_DATA_FORMAT,
            "version": PORTABLE_DATA_VERSION,
            "templates": [
                { "name": "Local", "content": template_content() },
                { "name": "Local", "content": "invalid" }
            ],
            "profiles": []
        })
        .to_string();

        let plan = plan_import(&content, &config).expect("archive parses");
        assert_eq!(plan.report.templates.skipped, 1);
        assert_eq!(plan.report.templates.overwritten, 0);
        assert_eq!(plan.config.templates[0].name, "Local");
    }

    #[test]
    fn overwrite_by_name_keeps_id_rebuilds_metadata_and_preserves_selection() {
        let mut config = AppConfig::default();
        config.templates.push(ProfileTemplate {
            id: String::from("template-one"),
            name: String::from("Default"),
            content: template_content(),
            updated_at: 1,
            reference_count: 0,
        });
        config
            .profiles
            .push(profile("profile-one", "Local", "template-one"));
        config.current_profile_id = Some(String::from("profile-one"));
        let content = json!({
            "format": PORTABLE_DATA_FORMAT,
            "version": PORTABLE_DATA_VERSION,
            "templates": [],
            "profiles": [{
                "name": "Local",
                "template_name": "Default",
                "inline_template": null,
                "remotes": [],
                "hook": null,
                "update_interval_hours": 12,
                "update_cron": null
            }]
        })
        .to_string();

        let plan = plan_import(&content, &config).expect("archive parses");
        let imported = &plan.config.profiles[0];
        assert_eq!(plan.config.current_profile_id.as_deref(), Some("profile-one"));
        assert_eq!(plan.report.profiles.overwritten, 1);
        assert_eq!(plan.report.profiles.warnings, 1);
        assert_eq!(imported.id, "profile-one");
        assert_eq!(imported.name, "Local");
        assert_eq!(imported.revision, 8);
        assert_eq!(imported.last_attempt_at, 0);
        assert!(imported.last_update_error.is_none());
        assert!(imported.next_update_at > imported.updated_at);
    }
}
