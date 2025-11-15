//! # Phase 11: Async Executor and Runtime
//!
//! Provides basic async task execution infrastructure.
//!
//! ## Components
//! - **Task**: Individual async task with state and waker
//! - **Executor**: Polls and schedules tasks
//! - **Runtime**: Manages executor and event loop
//! - **TaskQueue**: Manages ready and pending task queues

use crate::runtime::async_types::{Context, Poll};
use std::collections::{HashMap, VecDeque};
use std::fmt;

/// Unique identifier for async tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(pub usize);

impl TaskId {
    pub fn new(id: usize) -> Self {
        TaskId(id)
    }

    pub fn next(self) -> Self {
        TaskId(self.0 + 1)
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Task({})", self.0)
    }
}

/// State of an async task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to execute
    Ready,
    /// Task is waiting for something
    Waiting,
    /// Task has completed
    Completed,
    /// Task encountered an error
    Failed,
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TaskState::Ready => write!(f, "Ready"),
            TaskState::Waiting => write!(f, "Waiting"),
            TaskState::Completed => write!(f, "Completed"),
            TaskState::Failed => write!(f, "Failed"),
        }
    }
}

/// Represents a single async task
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,
    /// Current state of the task
    pub state: TaskState,
    /// Number of times this task has been polled
    pub poll_count: usize,
    /// Whether this task was awakened
    pub awakened: bool,
    /// Task context (waker, etc.)
    pub context: Context,
}

impl Task {
    pub fn new(id: TaskId) -> Self {
        Task {
            id,
            state: TaskState::Ready,
            poll_count: 0,
            awakened: false,
            context: Context::new(id.0),
        }
    }

    pub fn poll(&mut self) -> Poll<TaskState> {
        self.poll_count += 1;

        match self.state {
            TaskState::Completed | TaskState::Failed => Poll::Ready(self.state),
            TaskState::Ready if self.awakened => {
                self.awakened = false;
                Poll::Ready(TaskState::Ready)
            }
            _ => Poll::Pending,
        }
    }

    pub fn wake(&mut self) {
        self.awakened = true;
    }

    pub fn is_done(&self) -> bool {
        self.state == TaskState::Completed || self.state == TaskState::Failed
    }

    pub fn mark_completed(&mut self) {
        self.state = TaskState::Completed;
    }

    pub fn mark_failed(&mut self) {
        self.state = TaskState::Failed;
    }

    pub fn mark_waiting(&mut self) {
        self.state = TaskState::Waiting;
    }

    pub fn poll_times(&self) -> usize {
        self.poll_count
    }
}

/// Error type for executor operations
#[derive(Debug, Clone)]
pub struct ExecutorError {
    pub message: String,
    pub kind: ExecutorErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorErrorKind {
    TaskNotFound,
    NoTasksReady,
    ExecutionFailed,
    QueueFull,
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExecutorError: {}", self.message)
    }
}

pub type ExecutorResult<T> = Result<T, ExecutorError>;

/// Task queue managing ready and pending tasks
#[derive(Debug, Clone)]
pub struct TaskQueue {
    ready_tasks: VecDeque<TaskId>,
    waiting_tasks: Vec<TaskId>,
    max_size: usize,
}

impl TaskQueue {
    pub fn new(max_size: usize) -> Self {
        TaskQueue {
            ready_tasks: VecDeque::new(),
            waiting_tasks: Vec::new(),
            max_size,
        }
    }

    pub fn enqueue_ready(&mut self, task_id: TaskId) -> ExecutorResult<()> {
        if self.ready_tasks.len() >= self.max_size {
            return Err(ExecutorError {
                message: "ready queue is full".to_string(),
                kind: ExecutorErrorKind::QueueFull,
            });
        }
        self.ready_tasks.push_back(task_id);
        Ok(())
    }

    pub fn enqueue_waiting(&mut self, task_id: TaskId) -> ExecutorResult<()> {
        if self.waiting_tasks.len() >= self.max_size {
            return Err(ExecutorError {
                message: "waiting queue is full".to_string(),
                kind: ExecutorErrorKind::QueueFull,
            });
        }
        self.waiting_tasks.push(task_id);
        Ok(())
    }

    pub fn dequeue_ready(&mut self) -> Option<TaskId> {
        self.ready_tasks.pop_front()
    }

    pub fn pop_waiting(&mut self) -> Option<TaskId> {
        if self.waiting_tasks.is_empty() {
            None
        } else {
            Some(self.waiting_tasks.remove(0))
        }
    }

    pub fn ready_count(&self) -> usize {
        self.ready_tasks.len()
    }

    pub fn waiting_count(&self) -> usize {
        self.waiting_tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ready_tasks.is_empty() && self.waiting_tasks.is_empty()
    }
}

/// Basic async executor
pub struct Executor {
    /// All tasks indexed by ID
    tasks: HashMap<TaskId, Task>,
    /// Task queue
    queue: TaskQueue,
    /// Next task ID to allocate
    next_task_id: usize,
    /// Total tasks executed
    tasks_executed: usize,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: HashMap::new(),
            queue: TaskQueue::new(1000),
            next_task_id: 0,
            tasks_executed: 0,
        }
    }

    /// Spawn a new task
    pub fn spawn(&mut self) -> ExecutorResult<TaskId> {
        let task_id = TaskId::new(self.next_task_id);
        self.next_task_id += 1;

        let task = Task::new(task_id);
        self.tasks.insert(task_id, task);
        self.queue.enqueue_ready(task_id)?;

        Ok(task_id)
    }

    /// Get a task's current state
    pub fn get_task_state(&self, task_id: TaskId) -> ExecutorResult<TaskState> {
        self.tasks
            .get(&task_id)
            .map(|t| t.state)
            .ok_or(ExecutorError {
                message: format!("task {} not found", task_id.0),
                kind: ExecutorErrorKind::TaskNotFound,
            })
    }

    /// Poll the next ready task
    pub fn poll_next(&mut self) -> ExecutorResult<Option<TaskId>> {
        let task_id = match self.queue.dequeue_ready() {
            Some(id) => id,
            None => {
                if self.queue.waiting_count() > 0 {
                    return Err(ExecutorError {
                        message: "no ready tasks".to_string(),
                        kind: ExecutorErrorKind::NoTasksReady,
                    });
                }
                return Ok(None);
            }
        };

        let task = self.tasks.get_mut(&task_id).ok_or(ExecutorError {
            message: format!("task {} not found", task_id.0),
            kind: ExecutorErrorKind::TaskNotFound,
        })?;

        let result = task.poll();

        match result {
            Poll::Ready(TaskState::Completed) | Poll::Ready(TaskState::Failed) => {
                self.tasks_executed += 1;
                Ok(Some(task_id))
            }
            Poll::Ready(_) => {
                self.queue.enqueue_ready(task_id)?;
                Ok(Some(task_id))
            }
            Poll::Pending => {
                self.queue.enqueue_waiting(task_id)?;
                Ok(Some(task_id))
            }
        }
    }

    /// Run executor until all tasks complete
    pub fn run_until_complete(&mut self) -> ExecutorResult<usize> {
        let start_tasks = self.tasks.len();

        while !self.queue.is_empty() {
            self.poll_next()?;
        }

        Ok(start_tasks)
    }

    /// Wake a specific task
    pub fn wake_task(&mut self, task_id: TaskId) -> ExecutorResult<()> {
        let task = self.tasks.get_mut(&task_id).ok_or(ExecutorError {
            message: format!("task {} not found", task_id.0),
            kind: ExecutorErrorKind::TaskNotFound,
        })?;

        task.wake();

        let waiting_idx = self
            .queue
            .waiting_tasks
            .iter()
            .position(|&id| id == task_id);

        if let Some(idx) = waiting_idx {
            self.queue.waiting_tasks.remove(idx);
            self.queue.enqueue_ready(task_id)?;
        }

        Ok(())
    }

    /// Get executor statistics
    pub fn stats(&self) -> ExecutorStats {
        ExecutorStats {
            total_tasks: self.tasks.len(),
            ready_count: self.queue.ready_count(),
            waiting_count: self.queue.waiting_count(),
            completed_tasks: self.tasks_executed,
        }
    }

    /// Get number of tasks executed
    pub fn tasks_executed(&self) -> usize {
        self.tasks_executed
    }
}

/// Statistics about executor state
#[derive(Debug, Clone)]
pub struct ExecutorStats {
    pub total_tasks: usize,
    pub ready_count: usize,
    pub waiting_count: usize,
    pub completed_tasks: usize,
}

impl fmt::Display for ExecutorStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Executor: {} total, {} ready, {} waiting, {} completed",
            self.total_tasks, self.ready_count, self.waiting_count, self.completed_tasks
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_creation() {
        let id1 = TaskId::new(0);
        let id2 = TaskId::new(1);
        assert_ne!(id1, id2);
        assert_eq!(id1.0, 0);
        assert_eq!(id2.0, 1);
    }

    #[test]
    fn test_task_id_next() {
        let id = TaskId::new(5);
        let next = id.next();
        assert_eq!(next.0, 6);
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new(TaskId::new(0));
        assert_eq!(task.state, TaskState::Ready);
        assert_eq!(task.poll_count, 0);
        assert!(!task.awakened);
    }

    #[test]
    fn test_task_poll() {
        let mut task = Task::new(TaskId::new(0));
        assert_eq!(task.poll_count, 0);
        task.poll();
        assert_eq!(task.poll_count, 1);
    }

    #[test]
    fn test_task_mark_completed() {
        let mut task = Task::new(TaskId::new(0));
        task.mark_completed();
        assert_eq!(task.state, TaskState::Completed);
    }

    #[test]
    fn test_task_queue_enqueue_ready() {
        let mut queue = TaskQueue::new(10);
        let id = TaskId::new(0);
        assert!(queue.enqueue_ready(id).is_ok());
        assert_eq!(queue.ready_count(), 1);
    }

    #[test]
    fn test_task_queue_dequeue_ready() {
        let mut queue = TaskQueue::new(10);
        let id = TaskId::new(0);
        queue.enqueue_ready(id).unwrap();
        assert_eq!(queue.dequeue_ready(), Some(id));
        assert_eq!(queue.ready_count(), 0);
    }

    #[test]
    fn test_executor_spawn() {
        let mut executor = Executor::new();
        let task_id = executor.spawn().unwrap();
        assert_eq!(task_id.0, 0);
    }

    #[test]
    fn test_executor_spawn_multiple() {
        let mut executor = Executor::new();
        let id1 = executor.spawn().unwrap();
        let id2 = executor.spawn().unwrap();
        let id3 = executor.spawn().unwrap();

        assert_eq!(id1.0, 0);
        assert_eq!(id2.0, 1);
        assert_eq!(id3.0, 2);
    }

    #[test]
    fn test_executor_get_task_state() {
        let mut executor = Executor::new();
        let task_id = executor.spawn().unwrap();
        let state = executor.get_task_state(task_id).unwrap();
        assert_eq!(state, TaskState::Ready);
    }

    #[test]
    fn test_executor_get_nonexistent_task() {
        let executor = Executor::new();
        let result = executor.get_task_state(TaskId::new(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_executor_poll_next() {
        let mut executor = Executor::new();
        executor.spawn().unwrap();
        let result = executor.poll_next().unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_executor_stats() {
        let mut executor = Executor::new();
        executor.spawn().unwrap();
        executor.spawn().unwrap();

        let stats = executor.stats();
        assert_eq!(stats.total_tasks, 2);
        assert_eq!(stats.ready_count, 2);
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(format!("{}", TaskState::Ready), "Ready");
        assert_eq!(format!("{}", TaskState::Waiting), "Waiting");
        assert_eq!(format!("{}", TaskState::Completed), "Completed");
        assert_eq!(format!("{}", TaskState::Failed), "Failed");
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskId::new(5);
        let display = format!("{}", id);
        assert!(display.contains("5"));
    }

    #[test]
    fn test_task_wake() {
        let mut task = Task::new(TaskId::new(0));
        assert!(!task.awakened);
        task.wake();
        assert!(task.awakened);
    }

    #[test]
    fn test_task_is_done() {
        let mut task = Task::new(TaskId::new(0));
        assert!(!task.is_done());
        task.mark_completed();
        assert!(task.is_done());
    }

    #[test]
    fn test_executor_wake_task() {
        let mut executor = Executor::new();
        let task_id = executor.spawn().unwrap();
        assert!(executor.wake_task(task_id).is_ok());
    }

    #[test]
    fn test_task_queue_full() {
        let mut queue = TaskQueue::new(2);
        queue.enqueue_ready(TaskId::new(0)).unwrap();
        queue.enqueue_ready(TaskId::new(1)).unwrap();
        let result = queue.enqueue_ready(TaskId::new(2));
        assert!(result.is_err());
    }
}
