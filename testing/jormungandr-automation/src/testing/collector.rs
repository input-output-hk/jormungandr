use std::{
    io::{BufRead, Read},
    sync::mpsc::{self, Receiver},
    time::Instant,
};

pub struct OutputCollector {
    rx: Receiver<(Instant, String)>,
    collected: Vec<String>,
}

impl OutputCollector {
    pub fn new<R: Read + Send + 'static>(source: R) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let lines = std::io::BufReader::new(source).lines();
            for line in lines {
                tx.send((Instant::now(), line.unwrap())).unwrap();
            }
        });
        Self {
            rx,
            collected: Vec::new(),
        }
    }

    fn read_input(&mut self) {
        let now = Instant::now();
        while let Ok((time, line)) = self.rx.try_recv() {
            self.collected.push(line);
            // Stop reading if the are more recent messages available, otherwise
            // we risk that a very active process could result in endless collection
            // of its output
            if time > now {
                break;
            }
        }
    }

    /// Collecte available input up to the point in time when
    /// this function was called and take collected lines out of
    /// the collector
    pub fn take_available_input(&mut self) -> Vec<String> {
        self.read_input();
        std::mem::take(&mut self.collected)
    }

    /// Collected available input up to the point in time when
    /// this function was called and return a reference to the lines
    /// collected by this collector
    pub fn get_available_input(&mut self) -> &[String] {
        self.read_input();
        &self.collected
    }
}
