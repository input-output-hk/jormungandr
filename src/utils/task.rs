use std::thread;
use std::clone::Clone;
use std::sync::mpsc::{Sender, Receiver, channel};

#[allow(dead_code)]
pub struct Task {
    handler: thread::JoinHandle<()>,
    name: &'static str,
}

#[allow(dead_code)]
pub struct TaskWithInputs<A> {
    task: Task,
    channel_input: TaskMessageBox<A>,
}

pub struct Tasks {
    all_tasks: Vec<Task>
}
impl Tasks {
    pub fn new() -> Self {
        Tasks {
            all_tasks: Vec::new(),
        }
    }

    pub fn task_create<F>(&mut self, name: &'static str, f: F)
      where F: FnOnce() -> (),
            F: Send + 'static,
    {
        let handler = thread::spawn(move || { f() });
        let task = Task {
            handler: handler,
            name: name,
        };
        self.all_tasks.push(task);
    }

    pub fn task_create_with_inputs<F, A>(&mut self, name: &'static str, f: F) -> TaskMessageBox<A>
      where F: FnOnce(Receiver<A>) -> (),
            F: Send + 'static,
            A: Send + 'static,
    {
        let (tx, rx) = channel();

        self.task_create(name, move || { f(rx) });

        TaskMessageBox(tx)
    }

    pub fn join(self) {
        for thread in self.all_tasks {
            // TODO
            thread.handler.join();
        }
    }
}


#[derive(Clone)]
pub struct TaskMessageBox<A>(Sender<A>);

impl<A> TaskMessageBox<A> {
    pub fn send_to(self, a: A) {
        self.0.send(a).unwrap()
    }
}

impl<A> TaskWithInputs<A> {
    pub fn get_message_box(&self) -> TaskMessageBox<A> {
        TaskMessageBox(self.channel_input.0.clone())
    }
}
