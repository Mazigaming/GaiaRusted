// Threads, and the ways they share data: Arc, Mutex, RwLock, atomics,
// channels, condition variables, barriers and scoped threads.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Barrier, Condvar, Mutex, OnceLock, RwLock};
use std::thread;
use std::time::Duration;

fn spawn_and_join() {
    let handles: Vec<_> = (0..6u64).map(|i| thread::spawn(move || (i, (1..=i).product::<u64>()))).collect();
    let factorials: Vec<(u64, u64)> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
    println!("{:?}", factorials);
}

fn contended_counters() {
    let counter = Arc::new(Mutex::new(0u64));
    let hits = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let counter = Arc::clone(&counter);
        let hits = Arc::clone(&hits);
        handles.push(thread::spawn(move || {
            for _ in 0..2000 {
                *counter.lock().unwrap() += 1;
                hits.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    println!("{} {} {}", *counter.lock().unwrap(), hits.load(Ordering::SeqCst), Arc::strong_count(&counter));
}

fn channels() {
    let (sender, receiver) = mpsc::channel();
    for worker in 0..4 {
        let sender = sender.clone();
        thread::spawn(move || {
            for job in 0..3 {
                sender.send((worker, job, worker * 10 + job)).unwrap();
            }
        });
    }
    drop(sender);
    let mut results: Vec<(i32, i32, i32)> = receiver.iter().collect();
    results.sort();
    println!("{} {:?} {:?}", results.len(), results.first(), results.last());

    let (sender, receiver) = mpsc::channel::<String>();
    let consumer = thread::spawn(move || receiver.into_iter().map(|line| line.len()).sum::<usize>());
    for word in ["alpha", "beta", "gamma"] {
        sender.send(word.to_string()).unwrap();
    }
    drop(sender);
    println!("{}", consumer.join().unwrap());
}

fn scoped() {
    let mut numbers = vec![1u64, 2, 3, 4, 5, 6, 7, 8];
    let total: u64 = thread::scope(|scope| {
        let (left, right) = numbers.split_at(4);
        let first = scope.spawn(move || left.iter().sum::<u64>());
        let second = scope.spawn(move || right.iter().sum::<u64>());
        first.join().unwrap() + second.join().unwrap()
    });
    thread::scope(|scope| {
        for chunk in numbers.chunks_mut(3) {
            scope.spawn(move || chunk.iter_mut().for_each(|n| *n *= 10));
        }
    });
    println!("{} {:?}", total, numbers);
}

fn locks_and_signals() {
    let table = Arc::new(RwLock::new(HashMap::new()));
    {
        let mut writer = table.write().unwrap();
        writer.insert("answer", 42);
    }
    let readers: Vec<_> = (0..3)
        .map(|_| {
            let table = Arc::clone(&table);
            thread::spawn(move || *table.read().unwrap().get("answer").unwrap())
        })
        .collect();
    let answers: Vec<i32> = readers.into_iter().map(|reader| reader.join().unwrap()).collect();
    println!("{:?}", answers);

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let signaller = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (ready, condition) = &*signaller;
        let mut ready = ready.lock().unwrap();
        while !*ready {
            ready = condition.wait(ready).unwrap();
        }
        "woken"
    });
    thread::sleep(Duration::from_millis(20));
    {
        let (ready, condition) = &*pair;
        *ready.lock().unwrap() = true;
        condition.notify_all();
    }
    println!("{}", waiter.join().unwrap());

    let barrier = Arc::new(Barrier::new(4));
    let arrived = Arc::new(AtomicUsize::new(0));
    let passed = Arc::new(AtomicBool::new(true));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (barrier, arrived, passed) = (Arc::clone(&barrier), Arc::clone(&arrived), Arc::clone(&passed));
            thread::spawn(move || {
                arrived.fetch_add(1, Ordering::SeqCst);
                let leader = barrier.wait().is_leader();
                if arrived.load(Ordering::SeqCst) != 4 {
                    passed.store(false, Ordering::SeqCst);
                }
                leader
            })
        })
        .collect();
    let leaders = handles.into_iter().filter(|_| true).map(|handle| handle.join().unwrap()).filter(|&leader| leader).count();
    println!("{} {}", leaders, passed.load(Ordering::SeqCst));

    static CONFIG: OnceLock<String> = OnceLock::new();
    let first = CONFIG.get_or_init(|| String::from("configured"));
    let second = CONFIG.get_or_init(|| String::from("ignored"));
    println!("{} {} {:?}", first, second, Duration::from_millis(1500));
}

fn main() {
    spawn_and_join();
    contended_counters();
    channels();
    scoped();
    locks_and_signals();
}
