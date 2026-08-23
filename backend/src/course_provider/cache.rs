use std::{collections::HashMap, time::Duration};

use tokio::time::Instant;

use super::CourseDetail;

const COURSE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_ENTRIES: usize = 256;

#[derive(Default)]
pub(super) struct Cache {
    courses: HashMap<String, CacheEntry<CourseDetail>>,
}

struct CacheEntry<T> {
    expires_at: Instant,
    value: T,
}

impl Cache {
    pub(super) fn course(&mut self, key: &str) -> Option<CourseDetail> {
        get(&mut self.courses, key)
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
        self.courses.retain(|_, entry| entry.expires_at > now);
        if self.courses.len() < MAX_ENTRIES {
            return;
        }
        if let Some(key) = self
            .courses
            .iter()
            .min_by_key(|(_, entry)| entry.expires_at)
            .map(|(key, _)| key.clone())
        {
            self.courses.remove(&key);
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
