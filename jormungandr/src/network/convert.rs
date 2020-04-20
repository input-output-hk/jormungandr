use crate::blockcfg::{Fragment, Header, HeaderId};
use chain_core::mempack::{ReadBuf, Readable};
use chain_core::property::Serialize;
use chain_network::data as net_data;
use chain_network::error::{Code, Error};

fn read<T, U>(src: &T) -> Result<U, Error>
where
    T: AsRef<[u8]>,
    U: Readable,
{
    let mut buf = ReadBuf::from(src.as_ref());
    U::read(&mut buf).map_err(|e| Error::new(Code::InvalidArgument, e))
}

fn read_vec<T, U>(src: &[T]) -> Result<Vec<U>, Error>
where
    T: AsRef<[u8]>,
    U: Readable,
{
    src.iter().map(|item| read(item)).collect()
}

pub trait TryFromNetwork<N> {
    fn try_from_network(data: N) -> Result<Self, Error>;
}

pub trait IntoNetwork<N> {
    fn into_network(self) -> N;
}

impl TryFromNetwork<net_data::Fragment> for Fragment {
    fn try_from_network(data: net_data::Fragment) -> Result<Self, Error> {
        read(&data)
    }
}
