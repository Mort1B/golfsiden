use std::{collections::HashSet, sync::OnceLock};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::course_provider::normalize_course_id;

const PROVIDER: &str = "golf_course_api";
const BUNDLED_CATALOG: &str = include_str!("../data/course_catalog.json");

static CATALOG: OnceLock<Result<CourseCatalog, String>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Usable,
    Incomplete,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CourseCatalogItem {
    pub display_name: String,
    pub country: String,
    pub provider: &'static str,
    pub provider_course_id: Option<String>,
    pub provider_status: ProviderStatus,
    pub provider_status_detail: String,
}

#[derive(Debug, Error)]
pub enum CourseCatalogError {
    #[error("bundled course catalog is invalid")]
    InvalidCatalog,
    #[error("catalog query must contain between 2 and 80 bytes without control characters")]
    InvalidQuery,
}

#[derive(Debug)]
pub enum ProviderCourseReadiness {
    Usable,
    Incomplete,
    Unknown,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogEntry {
    display_name: String,
    country: String,
    aliases: Vec<String>,
    provider_course_id: Option<String>,
    provider_status: ProviderStatus,
    provider_status_detail: String,
}

struct CourseCatalog {
    entries: Vec<CatalogEntry>,
}

pub fn search(query: Option<&str>) -> Result<Vec<CourseCatalogItem>, CourseCatalogError> {
    bundled()?.search(query)
}

pub fn provider_course_readiness(
    provider_course_id: &str,
) -> Result<ProviderCourseReadiness, CourseCatalogError> {
    let catalog = bundled()?;
    Ok(catalog
        .entries
        .iter()
        .find(|entry| entry.provider_course_id.as_deref() == Some(provider_course_id))
        .map_or(ProviderCourseReadiness::Unknown, |entry| {
            match entry.provider_status {
                ProviderStatus::Usable => ProviderCourseReadiness::Usable,
                ProviderStatus::Incomplete | ProviderStatus::Missing => {
                    ProviderCourseReadiness::Incomplete
                }
            }
        }))
}

fn bundled() -> Result<&'static CourseCatalog, CourseCatalogError> {
    CATALOG
        .get_or_init(|| CourseCatalog::parse(BUNDLED_CATALOG).map_err(|error| error.to_string()))
        .as_ref()
        .map_err(|_| CourseCatalogError::InvalidCatalog)
}

impl CourseCatalog {
    fn parse(json: &str) -> Result<Self, CourseCatalogError> {
        let entries: Vec<CatalogEntry> =
            serde_json::from_str(json).map_err(|_| CourseCatalogError::InvalidCatalog)?;
        if entries.is_empty() {
            return Err(CourseCatalogError::InvalidCatalog);
        }
        let mut names = HashSet::with_capacity(entries.len());
        let mut provider_ids = HashSet::with_capacity(entries.len());
        for entry in &entries {
            validate_text(&entry.display_name)?;
            validate_text(&entry.country)?;
            validate_text(&entry.provider_status_detail)?;
            if entry.aliases.is_empty() || !names.insert(entry.display_name.trim().to_lowercase()) {
                return Err(CourseCatalogError::InvalidCatalog);
            }
            for alias in &entry.aliases {
                validate_text(alias)?;
                if !names.insert(alias.trim().to_lowercase()) {
                    return Err(CourseCatalogError::InvalidCatalog);
                }
            }
            let normalized_id = entry
                .provider_course_id
                .as_deref()
                .map(normalize_course_id)
                .transpose()
                .map_err(|_| CourseCatalogError::InvalidCatalog)?;
            if normalized_id.as_deref() != entry.provider_course_id.as_deref()
                || normalized_id
                    .as_ref()
                    .is_some_and(|id| !provider_ids.insert(id.clone()))
                || matches!(entry.provider_status, ProviderStatus::Usable)
                    && normalized_id.is_none()
                || matches!(entry.provider_status, ProviderStatus::Missing)
                    && normalized_id.is_some()
            {
                return Err(CourseCatalogError::InvalidCatalog);
            }
        }
        Ok(Self { entries })
    }

    fn search(&self, query: Option<&str>) -> Result<Vec<CourseCatalogItem>, CourseCatalogError> {
        let query = normalize_query(query)?;
        Ok(self
            .entries
            .iter()
            .filter(|entry| {
                query.as_ref().is_none_or(|query| {
                    entry.display_name.to_lowercase().contains(query)
                        || entry
                            .aliases
                            .iter()
                            .any(|alias| alias.to_lowercase().contains(query))
                })
            })
            .map(CatalogEntry::to_item)
            .collect())
    }
}

impl CatalogEntry {
    fn to_item(&self) -> CourseCatalogItem {
        CourseCatalogItem {
            display_name: self.display_name.clone(),
            country: self.country.clone(),
            provider: PROVIDER,
            provider_course_id: self.provider_course_id.clone(),
            provider_status: self.provider_status,
            provider_status_detail: self.provider_status_detail.clone(),
        }
    }
}

fn normalize_query(value: Option<&str>) -> Result<Option<String>, CourseCatalogError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.len() > 80 || value.chars().any(char::is_control) {
        return Err(CourseCatalogError::InvalidQuery);
    }
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() < 2 {
        return Err(CourseCatalogError::InvalidQuery);
    }
    Ok(Some(value.to_lowercase()))
}

fn validate_text(value: &str) -> Result<(), CourseCatalogError> {
    if value.trim().is_empty() || value.len() > 500 || value.chars().any(char::is_control) {
        return Err(CourseCatalogError::InvalidCatalog);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_has_all_eight_entries_in_stable_order() {
        let courses = search(None).unwrap();
        let actual = courses
            .iter()
            .map(|course| {
                (
                    course.display_name.as_str(),
                    course.country.as_str(),
                    course.provider,
                    course.provider_course_id.as_deref(),
                    course.provider_status,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                (
                    "Hacienda del Álamo",
                    "Spain",
                    PROVIDER,
                    None,
                    ProviderStatus::Missing,
                ),
                (
                    "Saurines de la Torre",
                    "Spain",
                    PROVIDER,
                    None,
                    ProviderStatus::Missing,
                ),
                (
                    "Mar Menor",
                    "Spain",
                    PROVIDER,
                    None,
                    ProviderStatus::Missing,
                ),
                (
                    "Oppegård GK",
                    "Norway",
                    PROVIDER,
                    None,
                    ProviderStatus::Missing,
                ),
                (
                    "Drøbak GK",
                    "Norway",
                    PROVIDER,
                    None,
                    ProviderStatus::Missing,
                ),
                (
                    "Miklagard GK",
                    "Norway",
                    PROVIDER,
                    Some("0zm1pe1a"),
                    ProviderStatus::Incomplete,
                ),
                (
                    "Oslo GK",
                    "Norway",
                    PROVIDER,
                    Some("dcm3cn0g"),
                    ProviderStatus::Incomplete,
                ),
                (
                    "Haga GK",
                    "Norway",
                    PROVIDER,
                    Some("kcmzs8qz"),
                    ProviderStatus::Incomplete,
                ),
            ]
        );
    }

    #[test]
    fn search_handles_aliases_diacritics_case_and_blank_queries() {
        assert_eq!(
            search(Some("ALAMO")).unwrap()[0].display_name,
            "Hacienda del Álamo"
        );
        assert_eq!(
            search(Some("alamos")).unwrap()[0].display_name,
            "Hacienda del Álamo"
        );
        assert_eq!(
            search(Some("oppegard")).unwrap()[0].display_name,
            "Oppegård GK"
        );
        assert_eq!(search(Some("DRØBAK")).unwrap()[0].display_name, "Drøbak GK");
        assert_eq!(search(Some("golfklubb")).unwrap().len(), 5);
        assert_eq!(search(Some("  ")).unwrap().len(), 8);
    }

    #[test]
    fn query_validation_is_byte_bounded_and_rejects_controls() {
        assert!(matches!(
            search(Some("x")),
            Err(CourseCatalogError::InvalidQuery)
        ));
        assert!(matches!(
            search(Some(&"x".repeat(81))),
            Err(CourseCatalogError::InvalidQuery)
        ));
        assert!(matches!(
            search(Some("two\nlines")),
            Err(CourseCatalogError::InvalidQuery)
        ));
    }

    #[test]
    fn parser_rejects_inconsistent_provider_readiness() {
        let invalid = r#"[{"display_name":"Course","country":"Norway","aliases":["Alias"],"provider_course_id":null,"provider_status":"usable","provider_status_detail":"Ready"}]"#;
        assert!(CourseCatalog::parse(invalid).is_err());
    }

    #[test]
    fn parser_rejects_alias_collisions_across_entries() {
        let invalid = r#"[
            {"display_name":"First","country":"Norway","aliases":["Shared"],"provider_course_id":null,"provider_status":"missing","provider_status_detail":"Missing"},
            {"display_name":"Second","country":"Norway","aliases":["shared"],"provider_course_id":null,"provider_status":"missing","provider_status_detail":"Missing"}
        ]"#;
        assert!(CourseCatalog::parse(invalid).is_err());
    }

    #[test]
    fn parser_rejects_unknown_or_misspelled_fields() {
        let invalid = r#"[{
            "display_name":"Course",
            "country":"Norway",
            "aliases":["Alias"],
            "provider_course_id":null,
            "provider_status":"missing",
            "provider_status_details":"Misspelled field"
        }]"#;
        assert!(CourseCatalog::parse(invalid).is_err());
    }
}
