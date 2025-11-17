use std::collections::HashMap;
use crate::macros::{MacroDefinition, MacroPattern, TokenTree};
use std::time::Instant;

pub struct MacroExpansionStats {
    pub total_expansions: usize,
    pub successful_expansions: usize,
    pub failed_expansions: usize,
    pub total_time: std::time::Duration,
    pub cached_results: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl MacroExpansionStats {
    pub fn new() -> Self {
        MacroExpansionStats {
            total_expansions: 0,
            successful_expansions: 0,
            failed_expansions: 0,
            total_time: std::time::Duration::ZERO,
            cached_results: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn record_expansion(&mut self, success: bool, duration: std::time::Duration) {
        self.total_expansions += 1;
        if success {
            self.successful_expansions += 1;
        } else {
            self.failed_expansions += 1;
        }
        self.total_time += duration;
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.cache_hits as f64) / (total as f64)
        }
    }

    pub fn average_expansion_time(&self) -> std::time::Duration {
        if self.total_expansions == 0 {
            std::time::Duration::ZERO
        } else {
            self.total_time / self.total_expansions as u32
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_expansions == 0 {
            0.0
        } else {
            (self.successful_expansions as f64) / (self.total_expansions as f64)
        }
    }
}

pub struct MacroCache {
    cache: HashMap<String, std::vec::Vec<TokenTree>>,
    stats: MacroExpansionStats,
    max_size: usize,
}

impl MacroCache {
    pub fn new(max_size: usize) -> Self {
        MacroCache {
            cache: HashMap::new(),
            stats: MacroExpansionStats::new(),
            max_size,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<std::vec::Vec<TokenTree>> {
        if let Some(value) = self.cache.get(key) {
            self.stats.record_cache_hit();
            Some(value.clone())
        } else {
            self.stats.record_cache_miss();
            None
        }
    }

    pub fn insert(&mut self, key: String, value: std::vec::Vec<TokenTree>) {
        if self.cache.len() >= self.max_size {
            if let Some(first_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&first_key);
            }
        }
        self.cache.insert(key, value);
        self.stats.cached_results = self.cache.len();
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.stats.cached_results = 0;
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }

    pub fn stats(&self) -> &MacroExpansionStats {
        &self.stats
    }

    pub fn stats_mut(&mut self) -> &mut MacroExpansionStats {
        &mut self.stats
    }
}

pub struct MacroOptimizer {
    macro_defs: HashMap<String, MacroDefinition>,
    cache: MacroCache,
    pattern_cache: HashMap<String, std::vec::Vec<MacroPattern>>,
}

impl MacroOptimizer {
    pub fn new(cache_size: usize) -> Self {
        MacroOptimizer {
            macro_defs: HashMap::new(),
            cache: MacroCache::new(cache_size),
            pattern_cache: HashMap::new(),
        }
    }

    pub fn register_macro(&mut self, def: MacroDefinition) {
        let name = def.name.clone();
        self.macro_defs.insert(name, def);
    }

    pub fn get_macro(&self, name: &str) -> Option<&MacroDefinition> {
        self.macro_defs.get(name)
    }

    pub fn precompile_patterns(&mut self) {
        for (name, def) in &self.macro_defs {
            let patterns: std::vec::Vec<MacroPattern> = def.rules.iter()
                .flat_map(|rule| rule.pattern.iter().cloned())
                .collect();
            self.pattern_cache.insert(name.clone(), patterns);
        }
    }

    pub fn get_patterns(&self, name: &str) -> Option<&std::vec::Vec<MacroPattern>> {
        self.pattern_cache.get(name)
    }

    pub fn invalidate_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.cache.size()
    }

    pub fn stats(&self) -> &MacroExpansionStats {
        self.cache.stats()
    }
}

pub struct ExpansionProfiler {
    timings: HashMap<String, std::vec::Vec<std::time::Duration>>,
}

impl ExpansionProfiler {
    pub fn new() -> Self {
        ExpansionProfiler {
            timings: HashMap::new(),
        }
    }

    pub fn start_timer(&self) -> Instant {
        Instant::now()
    }

    pub fn record_expansion(&mut self, macro_name: String, duration: std::time::Duration) {
        self.timings.entry(macro_name)
            .or_insert_with(std::vec::Vec::new)
            .push(duration);
    }

    pub fn get_statistics(&self, macro_name: &str) -> Option<ExpansionStatistics> {
        self.timings.get(macro_name).map(|durations| {
            let count = durations.len();
            let total: std::time::Duration = durations.iter().sum();
            let min = *durations.iter().min().unwrap_or(&std::time::Duration::ZERO);
            let max = *durations.iter().max().unwrap_or(&std::time::Duration::ZERO);
            let avg = if count > 0 {
                total / count as u32
            } else {
                std::time::Duration::ZERO
            };

            ExpansionStatistics {
                count,
                total,
                min,
                max,
                avg,
            }
        })
    }

    pub fn all_statistics(&self) -> HashMap<String, ExpansionStatistics> {
        self.timings.iter().map(|(name, durations)| {
            let count = durations.len();
            let total: std::time::Duration = durations.iter().sum();
            let min = *durations.iter().min().unwrap_or(&std::time::Duration::ZERO);
            let max = *durations.iter().max().unwrap_or(&std::time::Duration::ZERO);
            let avg = if count > 0 {
                total / count as u32
            } else {
                std::time::Duration::ZERO
            };

            (name.clone(), ExpansionStatistics {
                count,
                total,
                min,
                max,
                avg,
            })
        }).collect()
    }

    pub fn clear(&mut self) {
        self.timings.clear();
    }
}

pub struct ExpansionStatistics {
    pub count: usize,
    pub total: std::time::Duration,
    pub min: std::time::Duration,
    pub max: std::time::Duration,
    pub avg: std::time::Duration,
}

pub fn measure_expansion<F, R>(f: F) -> (R, std::time::Duration)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_cache_creation() {
        let cache = MacroCache::new(100);
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.max_size(), 100);
    }

    #[test]
    fn test_macro_cache_insert_and_get() {
        let mut cache = MacroCache::new(100);
        let tokens = vec![];
        cache.insert("test".to_string(), tokens.clone());

        let retrieved = cache.get("test");
        assert!(retrieved.is_some());
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn test_macro_cache_miss() {
        let mut cache = MacroCache::new(100);
        let result = cache.get("nonexistent");
        assert!(result.is_none());
        assert_eq!(cache.stats().cache_misses, 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = MacroCache::new(100);
        let tokens = vec![];
        cache.insert("key".to_string(), tokens);

        cache.get("key");
        cache.get("key");
        cache.get("nonexistent");

        let rate = cache.stats().cache_hit_rate();
        assert!(rate > 0.5 && rate < 1.0);
    }

    #[test]
    fn test_macro_cache_eviction() {
        let mut cache = MacroCache::new(2);
        cache.insert("key1".to_string(), vec![]);
        cache.insert("key2".to_string(), vec![]);
        cache.insert("key3".to_string(), vec![]);

        assert_eq!(cache.size(), 2);
    }

    #[test]
    fn test_macro_expansion_stats_success_rate() {
        let mut stats = MacroExpansionStats::new();
        stats.record_expansion(true, std::time::Duration::from_millis(1));
        stats.record_expansion(true, std::time::Duration::from_millis(1));
        stats.record_expansion(false, std::time::Duration::from_millis(1));

        assert_eq!(stats.success_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_macro_expansion_stats_average_time() {
        let mut stats = MacroExpansionStats::new();
        let duration = std::time::Duration::from_millis(10);
        stats.record_expansion(true, duration);
        stats.record_expansion(true, duration);

        let avg = stats.average_expansion_time();
        assert_eq!(avg, duration);
    }

    #[test]
    fn test_expansion_profiler_creation() {
        let profiler = ExpansionProfiler::new();
        let stats = profiler.get_statistics("nonexistent");
        assert!(stats.is_none());
    }

    #[test]
    fn test_expansion_profiler_record_and_get() {
        let mut profiler = ExpansionProfiler::new();
        let duration = std::time::Duration::from_millis(5);
        profiler.record_expansion("test_macro".to_string(), duration);

        let stats = profiler.get_statistics("test_macro");
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.count, 1);
        assert_eq!(s.total, duration);
        assert_eq!(s.avg, duration);
    }

    #[test]
    fn test_expansion_profiler_multiple_timings() {
        let mut profiler = ExpansionProfiler::new();
        let d1 = std::time::Duration::from_millis(1);
        let d2 = std::time::Duration::from_millis(2);
        let d3 = std::time::Duration::from_millis(3);

        profiler.record_expansion("macro".to_string(), d1);
        profiler.record_expansion("macro".to_string(), d2);
        profiler.record_expansion("macro".to_string(), d3);

        let stats = profiler.get_statistics("macro").unwrap();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min, d1);
        assert_eq!(stats.max, d3);
    }

    #[test]
    fn test_macro_optimizer_creation() {
        let optimizer = MacroOptimizer::new(50);
        assert_eq!(optimizer.cache_size(), 0);
    }

    #[test]
    fn test_macro_optimizer_register_macro() {
        let mut optimizer = MacroOptimizer::new(50);
        let def = MacroDefinition {
            name: "test".to_string(),
            rules: vec![],
        };
        optimizer.register_macro(def);

        let retrieved = optimizer.get_macro("test");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_measure_expansion_function() {
        let (result, duration) = measure_expansion(|| {
            let mut sum = 0;
            for i in 0..100 {
                sum += i;
            }
            sum
        });

        assert_eq!(result, 4950);
        assert!(duration.as_nanos() >= 0);
    }

    #[test]
    fn test_macro_cache_clear() {
        let mut cache = MacroCache::new(100);
        cache.insert("key1".to_string(), vec![]);
        cache.insert("key2".to_string(), vec![]);
        assert_eq!(cache.size(), 2);

        cache.clear();
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_expansion_profiler_clear() {
        let mut profiler = ExpansionProfiler::new();
        profiler.record_expansion("test".to_string(), std::time::Duration::from_millis(1));
        assert!(profiler.get_statistics("test").is_some());

        profiler.clear();
        assert!(profiler.get_statistics("test").is_none());
    }

    #[test]
    fn test_expansion_profiler_all_statistics() {
        let mut profiler = ExpansionProfiler::new();
        profiler.record_expansion("macro1".to_string(), std::time::Duration::from_millis(1));
        profiler.record_expansion("macro2".to_string(), std::time::Duration::from_millis(2));

        let all_stats = profiler.all_statistics();
        assert_eq!(all_stats.len(), 2);
        assert!(all_stats.contains_key("macro1"));
        assert!(all_stats.contains_key("macro2"));
    }

    #[test]
    fn test_macro_cache_size_limits() {
        let mut cache = MacroCache::new(3);
        for i in 0..5 {
            cache.insert(format!("key_{}", i), vec![]);
        }
        assert!(cache.size() <= 3);
    }

    #[test]
    fn test_expansion_statistics_calculations() {
        let mut profiler = ExpansionProfiler::new();
        for i in 1..=5 {
            profiler.record_expansion("test".to_string(), 
                std::time::Duration::from_millis(i));
        }

        let stats = profiler.get_statistics("test").unwrap();
        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, std::time::Duration::from_millis(1));
        assert_eq!(stats.max, std::time::Duration::from_millis(5));
    }
}
