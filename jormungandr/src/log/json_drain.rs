use slog::{Drain, OwnedKVList, Record};
use slog_json::Json;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

pub struct JsonDrain<D: Drain> {
    msg_buffer: SharedBuffer,
    json: Json<SharedBuffer>,
    drain: D,
}

impl<D: Drain> JsonDrain<D> {
    pub fn new(drain: D) -> Self {
        let msg_buffer = SharedBuffer::default();
        let json = Json::new(msg_buffer.clone())
            .set_newlines(false)
            .set_pretty(false)
            .add_default_keys()
            .build();
        JsonDrain {
            msg_buffer,
            json,
            drain,
        }
    }
}

impl<D: Drain> Drain for JsonDrain<D> {
    type Ok = D::Ok;
    type Err = D::Err;

    fn log(&self, record: &Record, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        self.json
            .log(record, values)
            .expect("JSON log formatting failed");
        let res = self.drain.log(
            &Record::new(
                &record_static!(record.level(), record.tag()),
                &format_args!("{}", self.msg_buffer),
                b!(),
            ),
            &o!().into(),
        );
        self.msg_buffer.clear();
        res
    }
}

#[derive(Clone, Default)]
struct SharedBuffer {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl SharedBuffer {
    fn clear(&self) {
        self.buffer.lock().unwrap().clear()
    }
}

impl Display for SharedBuffer {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            String::from_utf8_lossy(self.buffer.lock().unwrap().as_slice())
        )
    }
}

impl Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
