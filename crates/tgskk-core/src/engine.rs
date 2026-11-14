use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use crate::{Candidate, Dictionary};

#[derive(Clone, Debug)]
pub struct RankingRequest {
    generation: u64,
    reading: String,
    left_context: String,
    candidates: Arc<[Candidate]>,
}

impl RankingRequest {
    #[must_use]
    pub fn reading(&self) -> &str {
        &self.reading
    }
    #[must_use]
    pub fn left_context(&self) -> &str {
        &self.left_context
    }
    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    #[must_use]
    pub fn ranked(self, order: Vec<usize>) -> RankingResult {
        RankingResult {
            request: self,
            order,
        }
    }
}

pub struct RankingResult {
    request: RankingRequest,
    order: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateOutcome {
    Applied,
    Stale,
    Frozen,
    InvalidOrder,
}

struct Conversion {
    generation: u64,
    original: Arc<[Candidate]>,
    candidates: Arc<[Candidate]>,
    selected: usize,
    frozen: bool,
}

pub struct Engine {
    dictionary: Dictionary,
    cache: HashMap<String, Arc<[Candidate]>>,
    insertion_order: VecDeque<String>,
    cache_capacity: usize,
    generation: u64,
    conversion: Option<Conversion>,
}

impl Engine {
    #[must_use]
    pub fn new(dictionary: Dictionary, cache_capacity: usize) -> Self {
        Self {
            dictionary,
            cache: HashMap::new(),
            insertion_order: VecDeque::new(),
            cache_capacity,
            generation: 0,
            conversion: None,
        }
    }

    pub fn lookup(&mut self, reading: &str) -> Arc<[Candidate]> {
        if let Some(candidates) = self.cache.get(reading) {
            return candidates.clone();
        }
        let candidates = self.dictionary.lookup(reading);
        if self.cache_capacity != 0 {
            if self.cache.len() == self.cache_capacity
                && let Some(oldest) = self.insertion_order.pop_front()
            {
                self.cache.remove(&oldest);
            }
            self.insertion_order.push_back(reading.to_owned());
            self.cache.insert(reading.to_owned(), candidates.clone());
        }
        candidates
    }

    pub fn replace_dictionary(&mut self, dictionary: Dictionary) {
        self.dictionary = dictionary;
        self.cache.clear();
        self.insertion_order.clear();
        self.cancel();
    }

    pub fn begin(&mut self, reading: &str, left_context: &str) -> RankingRequest {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("conversion generation exhausted");
        let candidates = self.lookup(reading);
        self.conversion = Some(Conversion {
            generation: self.generation,
            original: candidates.clone(),
            candidates: candidates.clone(),
            selected: 0,
            frozen: false,
        });
        RankingRequest {
            generation: self.generation,
            reading: reading.to_owned(),
            left_context: left_context.to_owned(),
            candidates,
        }
    }

    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        self.conversion
            .as_ref()
            .map_or(&[], |conversion| &conversion.candidates)
    }

    pub fn apply_ranking(&mut self, result: RankingResult) -> UpdateOutcome {
        let Some(conversion) = self.conversion.as_mut() else {
            return UpdateOutcome::Stale;
        };
        if conversion.generation != result.request.generation
            || !Arc::ptr_eq(&conversion.original, &result.request.candidates)
        {
            return UpdateOutcome::Stale;
        }
        if conversion.frozen {
            return UpdateOutcome::Frozen;
        }
        let mut seen = vec![false; conversion.original.len()];
        if result.order.len() != seen.len() {
            return UpdateOutcome::InvalidOrder;
        }
        for &index in &result.order {
            let Some(used) = seen.get_mut(index) else {
                return UpdateOutcome::InvalidOrder;
            };
            if *used {
                return UpdateOutcome::InvalidOrder;
            }
            *used = true;
        }
        conversion.candidates = result
            .order
            .into_iter()
            .map(|index| conversion.original[index].clone())
            .collect::<Vec<_>>()
            .into();
        conversion.frozen = true;
        UpdateOutcome::Applied
    }

    pub fn select(&mut self, index: usize) -> Option<&Candidate> {
        let conversion = self.conversion.as_mut()?;
        if index >= conversion.candidates.len() {
            return None;
        }
        conversion.frozen = true;
        conversion.selected = index;
        conversion.candidates.get(index)
    }

    pub fn freeze(&mut self) {
        if let Some(conversion) = self.conversion.as_mut() {
            conversion.frozen = true;
        }
    }

    pub fn commit(&mut self) -> Option<Candidate> {
        let conversion = self.conversion.take()?;
        conversion.candidates.get(conversion.selected).cloned()
    }

    pub fn cancel(&mut self) {
        self.conversion = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_is_bounded_and_caches_misses() {
        let mut engine = Engine::new(Dictionary::default(), 2);
        engine.lookup("a");
        engine.lookup("b");
        engine.lookup("c");
        assert_eq!(engine.cache.len(), 2);
        assert!(!engine.cache.contains_key("a"));
        assert!(engine.cache["c"].is_empty());
        engine.replace_dictionary(Dictionary::default());
        assert!(engine.cache.is_empty());
        assert!(engine.insertion_order.is_empty());
    }
}
