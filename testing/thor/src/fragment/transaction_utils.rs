use chain_core::property::Serialize as _;
use chain_impl_mockchain::fragment::Fragment;

pub trait TransactionHash {
    fn encode(&self) -> String;
}

impl TransactionHash for Fragment {
    fn encode(&self) -> String {
        let bytes = self.serialize_as_vec().unwrap();
        hex::encode(&bytes)
    }
}
