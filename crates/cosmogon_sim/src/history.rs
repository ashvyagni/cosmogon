//! Meaningful events. The simulation never stores every tick — only what a historian would
//! write down, plus decimated per-civilization samples (see `civ::Samples`).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Astronomy,
    Life,
    Civilization,
    Technology,
    Disaster,
    War,
    Space,
    Contact,
}

impl Category {
    /// All eight categories, in the stable order used by `index()` and the Chronicle filters.
    pub const ALL: [Category; 8] = [
        Category::Astronomy,
        Category::Life,
        Category::Civilization,
        Category::Technology,
        Category::Disaster,
        Category::War,
        Category::Space,
        Category::Contact,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Astronomy => "Astronomy",
            Category::Life => "Life",
            Category::Civilization => "Civilization",
            Category::Technology => "Technology",
            Category::Disaster => "Disaster",
            Category::War => "War",
            Category::Space => "Space",
            Category::Contact => "Contact",
        }
    }

    /// Stable filter slot: Astronomy = 0 … Contact = 7.
    pub fn index(self) -> usize {
        match self {
            Category::Astronomy => 0,
            Category::Life => 1,
            Category::Civilization => 2,
            Category::Technology => 3,
            Category::Disaster => 4,
            Category::War => 5,
            Category::Space => 6,
            Category::Contact => 7,
        }
    }

    /// Case-insensitive label lookup, for CLI arguments and other plain-text input.
    pub fn from_label(name: &str) -> Option<Category> {
        Self::ALL
            .into_iter()
            .find(|c| c.label().eq_ignore_ascii_case(name))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Event {
    pub time: f64,
    pub category: Category,
    /// 1 (minor) ..= 5 (milestone).
    pub importance: u8,
    pub title: String,
    pub detail: String,
    pub system: Option<u32>,
    pub body: Option<u32>,
    pub civ: Option<u32>,
    /// Index into `History.events` of the event that caused this one; `None` = root cause
    /// (a user edit/intervention, or a natural event with no traced cause). Omitted from
    /// serialization when `None`, so old saves and golden-neutral runs are byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<usize>,
}

/// A Chronicle query: importance/category/text matching plus optional exact scope.
///
/// Every `Some` scope field must match the event exactly (logical AND); `None` means
/// "no restriction on this axis". This is read/derive-only — it never touches saved state.
#[derive(Clone, Debug, PartialEq)]
pub struct ChronicleFilter {
    /// Clamped to 1..=5 when matching; 1 (the default) keeps everything.
    pub min_importance: u8,
    /// Index = `Category::index()`; default all true.
    pub categories: [bool; 8],
    /// Case-insensitive substring over title + detail.
    pub text: Option<String>,
    pub system: Option<u32>,
    pub body: Option<u32>,
    pub civ: Option<u32>,
}

impl Default for ChronicleFilter {
    fn default() -> Self {
        Self {
            min_importance: 1,
            categories: [true; 8],
            text: None,
            system: None,
            body: None,
            civ: None,
        }
    }
}

impl ChronicleFilter {
    pub fn matches(&self, e: &Event) -> bool {
        if e.importance < self.min_importance.clamp(1, 5) {
            return false;
        }
        if !self.categories[e.category.index()] {
            return false;
        }
        if self.system.is_some_and(|s| e.system != Some(s)) {
            return false;
        }
        if self.body.is_some_and(|b| e.body != Some(b)) {
            return false;
        }
        if self.civ.is_some_and(|c| e.civ != Some(c)) {
            return false;
        }
        if let Some(needle) = self.text.as_ref().filter(|t| !t.is_empty()) {
            let needle = needle.to_lowercase();
            if !e.title.to_lowercase().contains(&needle) && !e.detail.to_lowercase().contains(&needle)
            {
                return false;
            }
        }
        true
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct History {
    pub events: Vec<Event>,
    /// Cached count of importance-<3 events, so `push` stays O(1) amortized. Not part of the
    /// save format: `serde(skip)` keeps the serialized bytes (and the golden fingerprint)
    /// byte-identical, and deserialization lazily recounts on the first push.
    #[serde(skip)]
    minor: Option<usize>,
    /// Index cursor of the oldest minor event, recomputed alongside `minor`. Not saved.
    #[serde(skip)]
    first_minor: Option<usize>,
}

/// The cache is an optimization, not semantic state: equality (used by save round-trip
/// assertions) compares the events only, exactly as the old derived `PartialEq` did.
impl PartialEq for History {
    fn eq(&self, other: &Self) -> bool {
        self.events == other.events
    }
}

impl History {
    /// Minor events beyond this count are pruned oldest-first; milestones are kept forever.
    pub const MINOR_CAP: usize = 4000;

    /// Push an event with an optional cause link. `push(e)` is `push_caused(e, None)`.
    pub fn push_caused(&mut self, e: Event, cause: Option<usize>) {
        let mut e = e;
        e.cause = cause;
        let minor = e.importance < 3;
        if self.minor.is_none() {
            self.recount();
        }
        self.events.push(e);
        if minor {
            let count = *self.minor.get_or_insert(0) + 1;
            self.minor = Some(count);
            if self.first_minor.is_none() {
                self.first_minor = Some(self.events.len() - 1);
            }
        }
        // One removal per push over the cap, exactly like the legacy full recount did.
        if self.minor.unwrap_or(0) > Self::MINOR_CAP {
            self.prune_one();
        }
    }

    /// Push an event with no cause (root / user-initiated). Backward-compatible wrapper
    /// around [`push_caused`].
    pub fn push(&mut self, e: Event) {
        self.push_caused(e, None);
    }

    /// Walk the `cause` chain from `start` back to its root, returning indices in
    /// chronological order (root first, `start` last). Stops at a `None` cause or an
    /// out-of-range index (e.g. after pruning removed the target). Cycles are impossible
    /// by construction (a cause always precedes its effect) but are defended against.
    pub fn why_trace(&self, start: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut idx = start;
        let mut seen = 0usize;
        while idx < self.events.len() {
            chain.push(idx);
            match self.events[idx].cause {
                Some(c) if c < self.events.len() => {
                    idx = c;
                    seen += 1;
                    if seen > self.events.len() {
                        // Defensive: should never happen; break rather than loop forever.
                        break;
                    }
                }
                _ => break,
            }
        }
        chain.reverse();
        chain
    }

    /// Forward links: indices of events whose `cause` points to `index`, in push order.
    pub fn cause_children(&self, index: usize) -> Vec<usize> {
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.cause == Some(index))
            .map(|(i, _)| i)
            .collect()
    }

    /// Rebuild both cache fields from the events vector (first push after `Default` or
    /// deserialization).
    fn recount(&mut self) {
        let mut count = 0usize;
        let mut first = None;
        for (i, e) in self.events.iter().enumerate() {
            if e.importance < 3 {
                count += 1;
                if first.is_none() {
                    first = Some(i);
                }
            }
        }
        self.minor = Some(count);
        self.first_minor = first;
    }

    /// Remove the oldest minor event. The cursor is rescanned forward only — each index is
    /// rescanned at most once across the run, so the cost amortizes to O(1) per push.
    fn prune_one(&mut self) {
        let mut i = self.first_minor.unwrap_or(0);
        while i < self.events.len() && self.events[i].importance >= 3 {
            i += 1;
        }
        if i >= self.events.len() {
            // Defensive: the cursor drifted (should be impossible while the count is right).
            i = match self.events.iter().position(|e| e.importance < 3) {
                Some(i) => i,
                None => return,
            };
        }
        self.events.remove(i);
        self.minor = Some(self.minor.unwrap_or(1).saturating_sub(1));
        let mut next = None;
        while i < self.events.len() {
            if self.events[i].importance < 3 {
                next = Some(i);
                break;
            }
            i += 1;
        }
        self.first_minor = next;
    }

    pub fn for_civ(&self, civ: u32) -> impl Iterator<Item = &Event> {
        self.events.iter().filter(move |e| e.civ == Some(civ))
    }

    /// Indices of the matching events in chronological order; same-time events keep their
    /// push (vector) index order, so the result is fully deterministic. The basis of
    /// `chronicle` and of the Legends view's index unions.
    pub fn chronicle_indices(&self, f: &ChronicleFilter) -> Vec<usize> {
        let mut idx: Vec<usize> = self
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| f.matches(e))
            .map(|(i, _)| i)
            .collect();
        idx.sort_by(|&a, &b| {
            self.events[a]
                .time
                .total_cmp(&self.events[b].time)
                .then(a.cmp(&b))
        });
        idx
    }

    /// Matching events in chronological order (ties broken by push index).
    pub fn chronicle(&self, f: &ChronicleFilter) -> Vec<&Event> {
        self.chronicle_indices(f)
            .into_iter()
            .map(|i| &self.events[i])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        time: f64,
        category: Category,
        importance: u8,
        title: &str,
        detail: &str,
        ids: (Option<u32>, Option<u32>, Option<u32>),
        cause: Option<usize>,
    ) -> Event {
        Event {
            time,
            category,
            importance,
            title: title.into(),
            detail: detail.into(),
            system: ids.0,
            body: ids.1,
            civ: ids.2,
            cause,
        }
    }

    fn sample() -> History {
        let mut h = History::default();
        h.push_caused(event(300.0, Category::War, 4, "The Great War", "Nations clash across the world.", (Some(0), Some(1), Some(7)), None), None);
        h.push_caused(event(100.0, Category::Astronomy, 5, "Comet passes", "A bright comet crosses the sky.", (Some(0), Some(1), None), None), None);
        h.push_caused(event(200.0, Category::Technology, 2, "Radio invented", "Wireless communication begins.", (Some(0), Some(2), Some(7)), None), None);
        h.push_caused(event(400.0, Category::Disaster, 3, "Pandemic", "Millions die of a new disease.", (Some(1), Some(0), None), None), None);
        h
    }

    fn titles(h: &History, f: &ChronicleFilter) -> Vec<String> {
        h.chronicle(f).into_iter().map(|e| e.title.clone()).collect()
    }

    #[test]
    fn chronicle_filters_by_importance_category_text_and_scope() {
        let h = sample();
        // Category::index is the stable filter slot: Astronomy = 0 … Contact = 7.
        assert_eq!(Category::Astronomy.index(), 0);
        assert_eq!(Category::Life.index(), 1);
        assert_eq!(Category::Civilization.index(), 2);
        assert_eq!(Category::Technology.index(), 3);
        assert_eq!(Category::Disaster.index(), 4);
        assert_eq!(Category::War.index(), 5);
        assert_eq!(Category::Space.index(), 6);
        assert_eq!(Category::Contact.index(), 7);

        // Default: everything, chronologically.
        assert_eq!(titles(&h, &ChronicleFilter::default()), ["Comet passes", "Radio invented", "The Great War", "Pandemic"]);

        // Importance, clamped into 1..=5 either side.
        let f = ChronicleFilter { min_importance: 4, ..Default::default() };
        assert_eq!(titles(&h, &f), ["Comet passes", "The Great War"]);
        let f = ChronicleFilter { min_importance: 0, ..Default::default() };
        assert_eq!(h.chronicle(&f).len(), 4, "0 clamps up to 1");
        let f = ChronicleFilter { min_importance: 9, ..Default::default() };
        assert_eq!(titles(&h, &f), ["Comet passes"], "9 clamps down to 5");

        // Category toggles.
        let mut f = ChronicleFilter::default();
        f.categories[Category::War.index()] = false;
        assert_eq!(titles(&h, &f), ["Comet passes", "Radio invented", "Pandemic"]);
        f.categories = [false; 8];
        f.categories[Category::Astronomy.index()] = true;
        assert_eq!(titles(&h, &f), ["Comet passes"]);

        // Case-insensitive substring over title and detail.
        let mut f = ChronicleFilter { text: Some("RADIO".into()), ..Default::default() };
        assert_eq!(titles(&h, &f), ["Radio invented"]);
        f.text = Some("nations clash".into());
        assert_eq!(titles(&h, &f), ["The Great War"], "detail text matches");
        f.text = Some("zeppelin".into());
        assert!(h.chronicle(&f).is_empty());
        f.text = Some(String::new());
        assert_eq!(h.chronicle(&f).len(), 4, "an empty search matches everything");

        // Scope: each Some field must match exactly (AND).
        let mut f = ChronicleFilter { system: Some(0), ..Default::default() };
        assert_eq!(titles(&h, &f), ["Comet passes", "Radio invented", "The Great War"]);
        f.body = Some(1);
        assert_eq!(titles(&h, &f), ["Comet passes", "The Great War"]);
        f.civ = Some(7);
        assert_eq!(titles(&h, &f), ["The Great War"]);
        f.system = Some(9);
        assert!(h.chronicle(&f).is_empty(), "an unknown system matches nothing");

        // The filter composes with everything at once.
        let mut f = ChronicleFilter { min_importance: 3, ..Default::default() };
        f.text = Some("the".into());
        f.system = Some(0);
        f.body = Some(1);
        assert_eq!(titles(&h, &f), ["Comet passes", "The Great War"]);
    }

    #[test]
    fn chronicle_is_deterministically_ordered_on_time_ties() {
        let mut h = History::default();
        h.push(event(20.0, Category::Life, 3, "b", "", (None, None, None), None));
        h.push(event(10.0, Category::War, 3, "x", "", (None, None, None), None));
        h.push(event(10.0, Category::War, 3, "y", "", (None, None, None), None));
        h.push(event(10.0, Category::War, 3, "z", "", (None, None, None), None));
        h.push(event(5.0, Category::Astronomy, 3, "a", "", (None, None, None), None));
        let expected = ["a", "x", "y", "z", "b"];
        let first = titles(&h, &ChronicleFilter::default());
        assert_eq!(first, expected, "ties keep push (vector) order");
        assert_eq!(titles(&h, &ChronicleFilter::default()), first, "repeated queries agree");
        // The index view used by the Legends union orders identically.
        let idx = h.chronicle_indices(&ChronicleFilter::default());
        assert_eq!(idx, [4, 1, 2, 3, 0]);
        assert_eq!(idx.iter().map(|&i| h.events[i].title.as_str()).collect::<Vec<_>>(), expected);
    }

    // ---- T4: the cached minor counter must evolve `events` exactly like the legacy recount ----

    /// Byte-for-byte copy of the pre-build-62 algorithm, for equivalence checking.
    fn legacy_push(h: &mut History, e: Event) {
        h.events.push(e);
        let minor = h.events.iter().filter(|e| e.importance < 3).count();
        if minor > History::MINOR_CAP {
            if let Some(i) = h.events.iter().position(|e| e.importance < 3) {
                h.events.remove(i);
            }
        }
    }

    fn mix(seed: &mut u64) -> u8 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = *seed >> 33;
        // 70% minor (importance 1..=2) so the 4000-minor cap is exercised within a few
        // thousand pushes; the rest spans 3..=5.
        if r % 10 < 7 {
            (r % 2 + 1) as u8
        } else {
            3 + (r % 3) as u8
        }
    }

    #[test]
    fn history_minor_counter_prunes_exactly_like_the_legacy_recount() {
        let mut fast = History::default();
        let mut slow = History::default();
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        for i in 0..7000 {
            let e = event(
                (i as f64) * 0.5,
                Category::War,
                mix(&mut seed),
                &format!("event {i}"),
                "detail",
                (Some(0), Some(1), None), None,
            );
            let reference = e.clone();
            fast.push(e);
            legacy_push(&mut slow, reference);
            assert_eq!(fast.events, slow.events, "divergence at push {i}");
        }
        // The cap held and pruning actually ran.
        assert!(slow.events.iter().filter(|e| e.importance < 3).count() <= History::MINOR_CAP);
        assert!(fast.events.iter().any(|e| e.importance < 3), "minors survive below the cap");
    }

    #[test]
    fn history_serialisation_is_byte_identical_with_the_counter_cached() {
        let mut warm = History::default();
        for i in 0..60 {
            warm.push(event(i as f64, Category::Life, (i % 5 + 1) as u8, "e", "d", (Some(0), Some(1), None), None));
        }
        let s1 = serde_json::to_string(&warm).unwrap();
        // The cache is not part of the save format in either direction.
        assert!(!s1.contains("\"minor\""));
        assert!(!s1.contains("\"first_minor\""));

        // Cold (deserialised) and warm caches continue identically.
        let mut cold: History = serde_json::from_str(&s1).unwrap();
        assert_eq!(serde_json::to_string(&cold).unwrap(), s1, "deserialise → serialise is byte-identical");
        let extra = event(99.0, Category::War, 2, "late", "minor event", (Some(0), Some(1), None), None);
        warm.push(extra.clone());
        cold.push(extra);
        let s2 = serde_json::to_string(&warm).unwrap();
        assert_eq!(serde_json::to_string(&cold).unwrap(), s2, "a cold cache continues byte-identically");
        assert_eq!(warm, cold, "manual PartialEq compares events only, so live == loaded stays true");
        // A default-constructed history still serialises to the bare save-format object.
        assert_eq!(serde_json::to_string(&History::default()).unwrap(), "{\"events\":[]}");
    }

    #[test]
    fn cause_links_form_a_chain_back_to_root() {
        let mut h = History::default();
        let root = h.events.len();
        h.push_caused(event(100.0, Category::Astronomy, 5, "Comet", "A bright comet crosses.", (Some(0), Some(1), None), None), None);
        h.push_caused(event(150.0, Category::Life, 4, "Life appears", "Biosphere emerges.", (Some(0), Some(1), None), None), Some(root));
        h.push_caused(event(200.0, Category::Civilization, 4, "Civ rises", "Settlements form.", (Some(0), Some(1), None), None), Some(root + 1));
        let trace = h.why_trace(root + 2);
        assert_eq!(trace, [root, root + 1, root + 2], "why_trace returns root→leaf");
    }

    #[test]
    fn why_trace_stops_at_root_or_pruned() {
        let mut h = History::default();
        h.push_caused(event(100.0, Category::War, 4, "Root", "First event.", (None, None, None), None), None);
        let leaf = h.events.len();
        h.push_caused(event(200.0, Category::War, 4, "Leaf", "Second event.", (None, None, None), None), Some(0));
        assert_eq!(h.why_trace(leaf), [0, leaf], "why_trace walks back through cause to root");
        assert_eq!(h.why_trace(0), [0]);
    }

    #[test]
    fn cause_children_returns_forward_links() {
        let mut h = History::default();
        h.push_caused(event(100.0, Category::Astronomy, 5, "Root", "First.", (None, None, None), None), None);
        h.push_caused(event(150.0, Category::Life, 4, "Child 1", "Second.", (None, None, None), None), Some(0));
        h.push_caused(event(200.0, Category::Civilization, 4, "Child 2", "Third.", (None, None, None), None), Some(0));
        h.push_caused(event(250.0, Category::Technology, 3, "Grandchild", "Fourth.", (None, None, None), None), Some(1));
        let children = h.cause_children(0);
        assert_eq!(children, [1, 2], "root's children are the two direct consequences");
        let child1 = h.cause_children(1);
        assert_eq!(child1, [3], "child 1 has one child");
        let child2 = h.cause_children(2);
        assert!(child2.is_empty(), "child 2 has no children");
    }

    #[test]
    fn cause_is_omitted_from_serialization_when_none() {
        let mut h = History::default();
        h.push_caused(event(100.0, Category::War, 4, "Root", "First.", (None, None, None), None), None);
        let s = serde_json::to_string(&h).unwrap();
        assert!(!s.contains("\"cause\""), "cause=None is omitted from JSON: {s}");
    }

    #[test]
    fn cause_is_preserved_in_serialization_when_some() {
        let mut h = History::default();
        h.push_caused(event(100.0, Category::Astronomy, 5, "Root", "First.", (None, None, None), None), None);
        let root_idx = h.events.len() - 1;
        h.push_caused(event(200.0, Category::Life, 4, "Child", "Second.", (None, None, None), None), Some(root_idx));
        let s = serde_json::to_string(&h).unwrap();
        assert!(s.contains("\"cause\""), "cause=Some is serialized: {s}");
        let h2: History = serde_json::from_str(&s).unwrap();
        assert_eq!(h2.events[1].cause, Some(root_idx), "cause survives round-trip");
        assert_eq!(h2.events[0].cause, None, "root cause is None after round-trip");
    }
}
