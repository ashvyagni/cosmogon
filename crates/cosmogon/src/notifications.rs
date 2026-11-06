//! App-only event routing. Chronicle events remain authoritative in Universe.
use bevy::prelude::*;
use cosmogon_sim::history::{Category, Event};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::sim::{Sim, Target, Toast};

pub const CATEGORIES: [Category; 8] = [Category::Astronomy, Category::Life, Category::Civilization, Category::Technology, Category::Disaster, Category::War, Category::Space, Category::Contact];
const CAPACITY: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity { Information, Notice, Warning, Critical }
impl Severity {
    pub const ALL: [Self; 4] = [Self::Information, Self::Notice, Self::Warning, Self::Critical];
    pub fn label(self) -> &'static str { match self { Self::Information => "Information", Self::Notice => "Notice", Self::Warning => "Warning", Self::Critical => "Critical" } }
    pub fn of(event: &Event) -> Self {
        // These labels route existing events, not a new physical hazard model.
        if matches!(event.category, Category::Disaster | Category::War) {
            if event.importance >= 5 { Self::Critical } else if event.importance >= 3 { Self::Warning } else { Self::Information }
        } else if event.importance >= 4 { Self::Notice } else { Self::Information }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    pub category: Category,
    pub popup: bool,
    pub center: bool,
    pub sound: bool,
    pub pause: bool,
    pub min_importance: u8,
}
impl Default for Rule {
    fn default() -> Self { Self { category: Category::Astronomy, popup: true, center: true, sound: true, pause: false, min_importance: 5 } }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationSettings {
    pub do_not_disturb: bool,
    pub mute_sounds: bool,
    pub critical_sound_only: bool,
    pub rules: Vec<Rule>,
}
impl Default for NotificationSettings {
    fn default() -> Self { Self { do_not_disturb: false, mute_sounds: false, critical_sound_only: false, rules: CATEGORIES.iter().map(|&category| Rule { category, ..Default::default() }).collect() } }
}
impl NotificationSettings {
    pub fn sanitize(&mut self) {
        self.rules = CATEGORIES.iter().map(|&category| {
            let mut rule = self.rules.iter().find(|r| r.category == category).cloned().unwrap_or(Rule { category, ..Default::default() });
            rule.min_importance = rule.min_importance.clamp(1, 5); rule
        }).collect();
    }
    pub fn rule(&self, category: Category) -> Rule { self.rules.iter().find(|r| r.category == category).cloned().unwrap_or(Rule { category, ..Default::default() }) }
    pub fn route(&self, event: &Event, photo: bool) -> Route {
        let r = self.rule(event.category);
        let eligible = event.importance >= r.min_importance;
        let quiet = self.do_not_disturb || photo;
        Route { center: r.center, popup: eligible && r.popup && !quiet,
            sound: eligible && r.sound && !quiet && !self.mute_sounds && (!self.critical_sound_only || Severity::of(event) == Severity::Critical),
            pause: eligible && r.pause && !quiet }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route { pub center: bool, pub popup: bool, pub sound: bool, pub pause: bool }
#[derive(Clone, Debug)]
pub struct Notice { pub event: Event, pub severity: Severity, pub read: bool }
impl Notice {
    pub fn target(&self, sim: &Sim) -> Option<Target> {
        let sid = self.event.system?;
        let sys = sim.universe.systems.get(sid as usize)?;
        match self.event.body {
            Some(body) if sys.bodies.get(body as usize).is_some_and(|b| b.exists()) => Some(Target::Body(cosmogon_sim::BodyRef { system: sid, body })),
            _ => Some(Target::Star(sid)),
        }
    }
}
#[derive(Resource)]
pub struct Notifications {
    pub entries: VecDeque<Notice>,
    generation: Option<u64>,
    tail: Option<Event>,
    last_bell: f64,
}
impl Default for Notifications {
    fn default() -> Self { Self { entries: VecDeque::new(), generation: None, tail: None, last_bell: -10.0 } }
}
impl Notifications {
    pub fn reset(&mut self) { *self = Self::default(); }
    pub fn unread(&self) -> usize { self.entries.iter().filter(|n| !n.read).count() }
    pub fn mark_read(&mut self) { for n in &mut self.entries { n.read = true; } }
    fn push(&mut self, event: Event, read: bool) {
        while self.entries.len() >= CAPACITY { self.entries.pop_front(); }
        self.entries.push_back(Notice { severity: Severity::of(&event), event, read });
    }
    /// Returns one coalesced alert cue. Generation changes import history silently.
    pub fn collect(&mut self, sim: &mut Sim, settings: &NotificationSettings, now: f64, photo: bool) -> bool {
        if self.generation != Some(sim.generation) {
            self.entries.clear();
            let history = &sim.universe.history.events;
            for e in history.iter().rev().take(CAPACITY).rev().filter(|e| settings.rule(e.category).center) { self.push(e.clone(), true); }
            self.tail = history.last().cloned();
            self.generation = Some(sim.generation);
            self.last_bell = now - 4.0;
            sim.toasts.clear();
            return false;
        }
        let history = &sim.universe.history.events;
        // Counts can stay constant when History prunes old minor events. Follow its
        // last event rather than relying only on length; rewinds reset generation.
        let start = self.tail.as_ref().and_then(|tail| history.iter().rposition(|e| e == tail)).map(|i| i + 1).unwrap_or(0);
        let new: Vec<_> = history[start..].to_vec();
        self.tail = history.last().cloned();
        let mut bell = false;
        for event in new {
            let route = settings.route(&event, photo);
            if route.popup {
                let notice = Notice { event: event.clone(), severity: Severity::of(&event), read: false };
                sim.toasts.push(Toast { title: event.title.clone(), detail: event.detail.clone(), target: notice.target(sim), born: now });
            }
            sim.paused |= route.pause;
            bell |= route.sound;
            if route.center { self.push(event, false); }
        }
        if settings.do_not_disturb || photo { sim.toasts.clear(); }
        sim.toasts.retain(|t| now - t.born < 9.0);
        let excess = sim.toasts.len().saturating_sub(4); sim.toasts.drain(..excess);
        if bell && now - self.last_bell >= 4.0 { self.last_bell = now; true } else { false }
    }
}
pub struct NotificationPlugin;
impl Plugin for NotificationPlugin {
    fn build(&self, app: &mut App) { app.init_resource::<Notifications>().add_systems(OnEnter(crate::state::AppState::Observing), begin); }
}

fn begin(mut n: ResMut<Notifications>, mut sim: ResMut<Sim>, settings: Res<crate::persistence::UserSettings>, time: Res<Time>) {
    n.reset();
    n.collect(&mut sim, &settings.notifications, time.elapsed_secs_f64(), false);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(time: f64, category: Category, importance: u8) -> Event { Event { time, category, importance, title: format!("Event {time}"), detail: "Test event".into(), system: Some(0), body: None, civ: None, cause: None } }
    fn sim() -> Sim { Sim::new(cosmogon_sim::Universe::new(cosmogon_sim::UniverseSettings { scenario: cosmogon_sim::Scenario::EmptySystem, ..Default::default() }), crate::sim::Session::Reference { dataset: "test".into(), title: "Test".into() }) }
    #[test]
    fn quiet_routes_retain_information_and_never_interrupt() {
        let e = event(1.0, Category::Disaster, 5);
        let mut s = NotificationSettings::default();
        s.rules.iter_mut().find(|r| r.category == Category::Disaster).unwrap().pause = true;
        assert_eq!(s.route(&e, false), Route { center: true, popup: true, sound: true, pause: true });
        assert_eq!(s.route(&e, true), Route { center: true, popup: false, sound: false, pause: false });
        s.do_not_disturb = true; assert_eq!(s.route(&e, false), s.route(&e, true));
    }
    #[test]
    fn critical_only_and_category_disable_work() {
        let mut s = NotificationSettings { critical_sound_only: true, ..Default::default() };
        assert!(!s.route(&event(1.0, Category::Technology, 5), false).sound);
        assert!(s.route(&event(1.0, Category::Disaster, 5), false).sound);
        s.rules.iter_mut().find(|r| r.category == Category::Disaster).unwrap().center = false;
        assert!(!s.route(&event(1.0, Category::Disaster, 5), false).center);
        s.mute_sounds = true; assert!(!s.route(&event(1.0, Category::Disaster, 5), false).sound);
    }
    #[test]
    fn pruning_rewind_and_bursts_preserve_model_and_reset_silently() {
        let mut sim = sim(); let mut n = Notifications::default(); let settings = NotificationSettings::default();
        sim.universe.history.events = vec![event(0.0, Category::Life, 2), event(1.0, Category::Life, 5)];
        assert!(!n.collect(&mut sim, &settings, 0.0, false)); assert_eq!(n.unread(), 0);
        sim.universe.history.events.remove(0); sim.universe.history.events.push(event(2.0, Category::Life, 5));
        let before = serde_json::to_string(&sim.universe).unwrap();
        assert!(n.collect(&mut sim, &settings, 5.0, false)); assert_eq!(n.unread(), 1); assert_eq!(sim.toasts.len(), 1);
        assert_eq!(serde_json::to_string(&sim.universe).unwrap(), before);
        assert!(!n.collect(&mut sim, &settings, 6.0, false));
        for i in 3..520 { sim.universe.history.events.push(event(i as f64, Category::Life, 5)); }
        assert!(n.collect(&mut sim, &settings, 10.0, false)); assert_eq!(n.entries.len(), CAPACITY); assert_eq!(sim.toasts.len(), 4);
        n.mark_read(); assert_eq!(n.unread(), 0);
        sim.generation += 1; assert!(!n.collect(&mut sim, &settings, 11.0, false)); assert!(sim.toasts.is_empty()); assert_eq!(n.unread(), 0);
    }
    #[test]
    fn old_settings_receive_complete_valid_routes() {
        let mut s: NotificationSettings = toml::from_str("mute_sounds = true").unwrap(); s.rules.clear(); s.sanitize(); assert_eq!(s.rules.len(), 8);
        s.rules[0].min_importance = 200; s.rules.push(s.rules[0].clone()); s.sanitize(); assert_eq!(s.rules.len(), 8); assert_eq!(s.rules[0].min_importance, 5);
        assert!(toml::from_str::<NotificationSettings>(&toml::to_string(&s).unwrap()).unwrap().mute_sounds);
    }
}
