use std::{collections::HashMap, time::Duration};

use tokio::time::Instant;

use super::{CourseDetail, CourseSearchResult};

const SEARCH_TTL: Duration = Duration::from_secs(600);
const COURSE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_ENTRIES: usize = 256;

#[derive(Default)]
pub(super) struct Cache {
    searches: HashMap<String, CacheEntry<Vec<CourseSearchResult>>>,
    courses: HashMap<String, CacheEntry<CourseDetail>>,
}

struct CacheEntry<T> {
    expires_at: Instant,
    value: T,
}

impl Cache {
    pub(super) fn search(&mut self, key: &str) -> Option<Vec<CourseSearchResult>> {
        get(&mut self.searches, key)
    }

    pub(super) fn course(&mut self, key: &str) -> Option<CourseDetail> {
        get(&mut self.courses, key)
    }

    pub(super) fn insert_search(&mut self, key: String, value: Vec<CourseSearchResult>) {
        self.prepare_insert();
        self.searches.insert(
            key,
            CacheEntry {
                expires_at: Instant::now() + SEARCH_TTL,
                value,
            },
        );
    }

    pub(super) fn insert_course(&mut self, key: String, value: CourseDetail) {
        self.prepare_insert();
        self.courses.insert(
            key,
            CacheEntry {
                expires_at: Instant::now() + COURSE_TTL,
                value,
            },
        );
    }

    fn prepare_insert(&mut self) {
        let now = Instant::now();
        self.searches.retain(|_, entry| entry.expires_at > now);
        self.courses.retain(|_, entry| entry.expires_at > now);
        if self.searches.len() + self.courses.len() < MAX_ENTRIES {
            return;
        }
        let oldest_search = self
            .searches
            .iter()
            .min_by_key(|(_, entry)| entry.expires_at)
            .map(|(key, entry)| (key.clone(), entry.expires_at));
        let oldest_course = self
            .courses
            .iter()
            .min_by_key(|(_, entry)| entry.expires_at)
            .map(|(key, entry)| (key.clone(), entry.expires_at));
        match (oldest_search, oldest_course) {
            (Some((key, search_expiry)), Some((_, course_expiry)))
                if search_expiry <= course_expiry =>
            {
                self.searches.remove(&key);
            }
            (_, Some((key, _))) => {
                self.courses.remove(&key);
            }
            (Some((key, _)), None) => {
                self.searches.remove(&key);
            }
            (None, None) => {}
        }
    }
}

fn get<T: Clone>(entries: &mut HashMap<String, CacheEntry<T>>, key: &str) -> Option<T> {
    let now = Instant::now();
    if entries
        .get(key)
        .is_some_and(|entry| entry.expires_at <= now)
    {
        entries.remove(key);
    }
    entries.get(key).map(|entry| entry.value.clone())
}
