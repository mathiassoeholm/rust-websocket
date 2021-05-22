use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: mpsc::Sender<Message>,
}

trait FnBox {
  fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
  fn call_box(self: Box<Self>) {
    (*self)()
  }
}

enum Message {
  NewJob(Job),
  Terminate,
}

type Job = Box<dyn FnBox + Send + 'static>;

// pub struct PoolCreationError;

impl ThreadPool {
  /// Create a new ThreadPool
  ///
  /// The size is the number of threads in the pool.
  ///
  /// # Panics
  ///
  /// The `new` function will panic if the size is zero.
  pub fn new(size: usize) -> ThreadPool {
    assert!(size > 0);

    let (sender, receiver) = mpsc::channel();

    let receiver = Arc::new(Mutex::new(receiver));

    let mut workers = Vec::with_capacity(4);

    for id in 0..size {
      workers.push(Worker::new(id, Arc::clone(&receiver)));
    }

    ThreadPool { workers, sender }
  }

  // pub fn alternative_new(size: usize) -> Result<ThreadPool, PoolCreationError> {
  //   if size > 0 {
  //     Ok(ThreadPool)
  //   } else {
  //     Err(PoolCreationError)
  //   }
  // }

  /// Executes the provided function on the next available threadpool worker
  ///
  /// # Examples
  ///
  /// ```
  /// use rust_websocket::ThreadPool;
  ///
  /// let pool = ThreadPool::new(4);
  /// pool.execute(|| { println!("doing some work"); });
  /// ```
  pub fn execute<F>(&self, f: F)
  where
    F: FnOnce() + Send + 'static,
  {
    let job = Box::new(f);

    self.sender.send(Message::NewJob(job)).unwrap();
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    println!("Sender terminate message to all workers.");

    for _ in &mut self.workers {
      self.sender.send(Message::Terminate).unwrap();
    }

    println!("Shutting down all workers.");

    for worker in &mut self.workers {
      println!("Shutting down worker {}", worker.id);

      if let Some(thread) = worker.thread.take() {
        thread.join().unwrap();
      }
    }
  }
}

struct Worker {
  id: usize,
  thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
  fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
    Worker {
      id,
      thread: Some(thread::spawn(move || loop {
        let message = receiver.lock().unwrap().recv().unwrap();
        match message {
          Message::NewJob(job) => {
            println!("Worker {} got a job; executing.", id);

            job.call_box();
          }
          Message::Terminate => {
            println!("Worker {} was told to terminate.", id);

            break;
          }
        }
      })),
    }
  }
}
