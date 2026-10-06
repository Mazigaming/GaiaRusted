//! Async executor for running async state machines
//!
//! Provides task scheduling, waker notifications, and event loop

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Task ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(usize);

impl TaskId {
    fn new(id: usize) -> Self {
        TaskId(id)
    }
}

/// Task state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to be polled
    Ready,
    /// Task is waiting for a wake notification
    Pending,
    /// Task has completed
    Completed,
}

/// A single async task
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub poll_count: usize,
}

impl Task {
    /// Create a new task
    pub fn new(id: TaskId) -> Self {
        Task {
            id,
            state: TaskState::Ready,
            poll_count: 0,
        }
    }

    /// Mark task as pending (waiting for wake)
    pub fn mark_pending(&mut self) {
        self.state = TaskState::Pending;
    }

    /// Mark task as completed
    pub fn mark_completed(&mut self) {
        self.state = TaskState::Completed;
    }

    /// Increment poll count
    pub fn increment_polls(&mut self) {
        self.poll_count += 1;
    }
}

/// Async task executor
pub struct Executor {
    /// Queue of tasks ready to be polled
    ready_tasks: VecDeque<TaskId>,
    /// All tasks indexed by ID
    tasks: std::collections::HashMap<TaskId, Task>,
    /// Next task ID to assign
    next_task_id: usize,
    /// Whether executor is running
    running: bool,
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Self {
        Executor {
            ready_tasks: VecDeque::new(),
            tasks: std::collections::HashMap::new(),
            next_task_id: 0,
            running: false,
        }
    }

    /// Spawn a new task
    pub fn spawn(&mut self) -> TaskId {
        let id = TaskId::new(self.next_task_id);
        self.next_task_id += 1;

        let task = Task::new(id);
        self.tasks.insert(id, task);
        self.ready_tasks.push_back(id);

        id
    }

    /// Get a task by ID
    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    /// Get a mutable task by ID
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(&id)
    }

    /// Mark a task as pending
    pub fn mark_pending(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.mark_pending();
        }
        // Remove from ready queue if it's there
        self.ready_tasks.retain(|&tid| tid != id);
    }

    /// Mark a task as completed and remove from queue
    pub fn mark_completed(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.mark_completed();
        }
    }

    /// Wake a pending task
    pub fn wake_task(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.get_mut(&id) {
            if task.state == TaskState::Pending {
                task.state = TaskState::Ready;
                self.ready_tasks.push_back(id);
            }
        }
    }

    /// Get the next ready task
    pub fn next_ready_task(&mut self) -> Option<TaskId> {
        self.ready_tasks.pop_front()
    }

    /// Poll a task
    pub fn poll_task(&mut self, id: TaskId) -> Result<bool, String> {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.increment_polls();
            
            match task.state {
                TaskState::Ready => {
                    // Task is ready to be polled
                    Ok(true)
                }
                TaskState::Pending => {
                    // Task is pending, shouldn't be polled yet
                    Ok(false)
                }
                TaskState::Completed => {
                    // Task is completed
                    Ok(false)
                }
            }
        } else {
            Err(format!("Task {:?} not found", id))
        }
    }

    /// Run the executor
    pub fn run(&mut self) -> Result<(), String> {
        self.running = true;

        // Keep polling until all tasks are done
        while !self.ready_tasks.is_empty() {
            if let Some(task_id) = self.next_ready_task() {
                // Poll the task
                self.poll_task(task_id)?;
                
                // In a real implementation, this would call the actual Future::poll
                // For now, we just mark it as completed
                self.mark_completed(task_id);
            }
        }

        self.running = false;
        Ok(())
    }

    /// Check if executor is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get total number of tasks
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get number of ready tasks
    pub fn ready_count(&self) -> usize {
        self.ready_tasks.len()
    }

    /// Get number of completed tasks
    pub fn completed_count(&self) -> usize {
        self.tasks.values().filter(|t| t.state == TaskState::Completed).count()
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple waker for task notification
#[derive(Clone)]
pub struct SimpleWaker {
    task_id: TaskId,
    executor: Arc<Mutex<Executor>>,
}

impl SimpleWaker {
    /// Create a new waker
    pub fn new(task_id: TaskId, executor: Arc<Mutex<Executor>>) -> Self {
        SimpleWaker {
            task_id,
            executor,
        }
    }

    /// Wake the associated task
    pub fn wake(&self) {
        if let Ok(mut executor) = self.executor.lock() {
            executor.wake_task(self.task_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = Executor::new();
        assert_eq!(executor.task_count(), 0);
        assert!(!executor.is_running());
    }

    #[test]
    fn test_spawn_task() {
        let mut executor = Executor::new();
        let id1 = executor.spawn();
        let id2 = executor.spawn();
        
        assert_eq!(executor.task_count(), 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_task_states() {
        let mut executor = Executor::new();
        let id = executor.spawn();
        
        let task = executor.get_task(id).unwrap();
        assert_eq!(task.state, TaskState::Ready);
        
        executor.mark_pending(id);
        let task = executor.get_task(id).unwrap();
        assert_eq!(task.state, TaskState::Pending);
        
        executor.mark_completed(id);
        let task = executor.get_task(id).unwrap();
        assert_eq!(task.state, TaskState::Completed);
    }

    #[test]
    fn test_wake_task() {
        let mut executor = Executor::new();
        let id = executor.spawn();
        
        // Initially ready
        assert_eq!(executor.ready_count(), 1);
        
        executor.mark_pending(id);
        assert_eq!(executor.ready_count(), 0);
        
        executor.wake_task(id);
        assert_eq!(executor.ready_count(), 1);
    }

    #[test]
    fn test_executor_run() {
        let mut executor = Executor::new();
        executor.spawn();
        executor.spawn();
        
        let result = executor.run();
        assert!(result.is_ok());
        assert_eq!(executor.completed_count(), 2);
    }

    #[test]
    fn test_poll_count() {
        let mut executor = Executor::new();
        let id = executor.spawn();
        
        let task = executor.get_task(id).unwrap();
        assert_eq!(task.poll_count, 0);
        
        executor.poll_task(id).ok();
        let task = executor.get_task(id).unwrap();
        assert_eq!(task.poll_count, 1);
    }
}
