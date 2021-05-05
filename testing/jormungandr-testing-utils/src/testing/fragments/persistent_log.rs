use bincode::Options;
use chain_core::property::Serialize;
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::interfaces::PersistentFragmentLog;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub struct PersistentLogViewer {
    persistent_fragment_logs: Vec<PersistentFragmentLog>,
}

impl PersistentLogViewer {
    pub fn new(dir: PathBuf) -> Result<Self, std::io::Error> {
        let mut persistent_fragment_logs = Vec::new();

        for path in std::fs::read_dir(&dir)?.map(|res| res.unwrap().path()) {
            let mut reader = BufReader::new(File::open(&path).unwrap());
            loop {
                let codec = bincode::DefaultOptions::new()
                    .with_fixint_encoding()
                    .allow_trailing_bytes();
                let decoded: PersistentFragmentLog = match codec.deserialize_from(&mut reader) {
                    Ok(data) => data,
                    Err(_err) => {
                        //TODO: add better handling of EOF
                        break;
                    }
                };
                persistent_fragment_logs.push(decoded);
            }
        }

        Ok(Self {
            persistent_fragment_logs,
        })
    }

    pub fn get_all(&self) -> Vec<Fragment> {
        self.persistent_fragment_logs
            .iter()
            .map(|x| x.fragment.clone())
            .collect()
    }

    pub fn get_bin(&self) -> Vec<Vec<u8>> {
        self.get_all()
            .iter()
            .map(|x| x.serialize_as_vec().unwrap())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.persistent_fragment_logs.len()
    }
}
