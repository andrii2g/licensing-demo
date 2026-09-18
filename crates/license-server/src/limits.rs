use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Limiter {
    entries: HashMap<String, (Instant, u32)>,
}
impl Limiter {
    pub fn allow(&mut self, key: String, max: u32) -> bool {
        self.allow_at(key, max, Instant::now())
    }
    fn allow_at(&mut self, key: String, max: u32, now: Instant) -> bool {
        self.entries
            .retain(|_, (t, _)| now.duration_since(*t) < Duration::from_secs(60));
        if !self.entries.contains_key(&key) && self.entries.len() >= 10000 {
            return false;
        }
        let e = self.entries.entry(key).or_insert((now, 0));
        if e.1 >= max {
            return false;
        }
        e.1 += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_rate_windows() {
        let now = Instant::now();
        let mut l = Limiter::default();
        assert!(l.allow_at("ip:test".into(), 1, now));
        assert!(!l.allow_at("ip:test".into(), 1, now));
        assert!(l.allow_at("id:test".into(), 1, now));
        assert!(l.allow_at("ip:test".into(), 1, now + Duration::from_secs(60)));
    }
}
