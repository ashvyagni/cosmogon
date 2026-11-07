//! Async generation.
//!
//! Background terrain and object generation so the main simulation thread
//! never blocks waiting for procedural content.

use std::collections::VecDeque;

/// A generation task: produce terrain/features for a region.
#[derive(Clone, Debug, PartialEq)]
pub struct GenerationTask {
    pub id: u64,
    pub region_x: i32,
    pub region_y: i32,
    pub priority: u32,
    pub completed: bool,
    pub result: Option<String>,
}

/// Async generation manager: queues and processes generation tasks.
#[derive(Clone, Debug)]
pub struct AsyncGenerationManager {
    pub queue: VecDeque<GenerationTask>,
    pub completed: VecDeque<GenerationTask>,
    pub next_id: u64,
    pub max_queue_size: usize,
}

impl AsyncGenerationManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            completed: VecDeque::new(),
            next_id: 0,
            max_queue_size: 1024,
        }
    }
    /// Submit a generation task. Returns the task id.
    pub fn submit(&mut self, region_x: i32, region_y: i32, priority: u32) -> u64 {
        if self.queue.len() >= self.max_queue_size {
            return u64::MAX; // queue full
        }
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push_back(GenerationTask {
            id,
            region_x,
            region_y,
            priority,
            completed: false,
            result: None,
        });
        // Sort by priority (higher first)
        self.queue.make_contiguous().sort_by(|a, b| b.priority.cmp(&a.priority));
        id
    }
    /// Process one task from the queue.
    pub fn process_one(&mut self) -> Option<u64> {
        if let Some(mut task) = self.queue.pop_front() {
            let id = task.id;
            task.completed = true;
            task.result = Some(format!("generated region ({},{})", task.region_x, task.region_y));
            self.completed.push_back(task);
            Some(id)
        } else {
            None
        }
    }
    /// Process all queued tasks.
    pub fn process_all(&mut self) -> Vec<u64> {
        let mut completed_ids = Vec::new();
        while let Some(id) = self.process_one() {
            completed_ids.push(id);
        }
        completed_ids
    }
    /// Get a completed task result by id.
    pub fn get_result(&self, id: u64) -> Option<&str> {
        self.completed.iter().find(|t| t.id == id).and_then(|t| t.result.as_deref())
    }
    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn task_submission_and_processing() {
        let mut mgr = AsyncGenerationManager::new();
        let id1 = mgr.submit(0, 0, 10);
        let id2 = mgr.submit(1, 1, 5);
        assert_eq!(mgr.queue_length(), 2);
        // Higher priority first
        let first = mgr.process_one().unwrap();
        assert_eq!(first, id1);
        let second = mgr.process_one().unwrap();
        assert_eq!(second, id2);
    }
    #[test] fn queue_full() {
        let mut mgr = AsyncGenerationManager::new();
        mgr.max_queue_size = 2;
        assert!(mgr.submit(0, 0, 1) != u64::MAX);
        assert!(mgr.submit(1, 1, 1) != u64::MAX);
        assert_eq!(mgr.submit(2, 2, 1), u64::MAX);
    }
    #[test] fn process_all() {
        let mut mgr = AsyncGenerationManager::new();
        mgr.submit(0, 0, 1);
        mgr.submit(1, 1, 1);
        let ids = mgr.process_all();
        assert_eq!(ids.len(), 2);
        assert_eq!(mgr.queue_length(), 0);
    }
}
