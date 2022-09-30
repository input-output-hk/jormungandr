use super::{
    node::{
        node_client::NodeClient, HandshakeRequest, HandshakeResponse, PullBlocksRequest,
        PullBlocksToTipRequest, PullHeadersRequest, TipRequest,
    },
    types::{Block, BlockIds, Fragment, FragmentIds, Header},
    watch::{
        watch_client::WatchClient, BlockSubscriptionRequest, SyncMultiverseRequest,
        TipSubscriptionRequest,
    },
};
use crate::jormungandr::grpc::read_into;
use chain_core::{
    packer::Codec,
    property::{FromStr, Serialize},
};
use chain_impl_mockchain::{
    block::Block as LibBlock,
    fragment::Fragment as LibFragment,
    header::{ChainLength, Header as LibHeader},
    key::Hash,
};
use futures::stream;
use std::{
    fmt,
    net::SocketAddr,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
use thiserror::Error;
use tokio::runtime::{Builder, Runtime};
use tonic::transport::Channel;

const CLIENT_RETRY_WAIT: Duration = Duration::from_millis(500);

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MockClientError {
    #[error("request failed with message '{0}'")]
    InvalidRequest(String),
    #[error(
        "could not parse address '{0}'. HINT: accepted format example: /ip4/127.0.0.1/tcp/9000"
    )]
    InvalidAddressFormat(String),
}

impl MockClientError {
    pub fn message(&self) -> String {
        format!("{}", self)
    }
}

pub struct JormungandrClient {
    addr: SocketAddr,
    inner_client: NodeClient<Channel>,
    rt: Runtime,
}

impl Clone for JormungandrClient {
    fn clone(&self) -> Self {
        JormungandrClient::new(self.addr)
    }
}

impl fmt::Debug for JormungandrClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JormungandrClient")
            .field("host", &self.addr)
            .finish()
    }
}

impl JormungandrClient {
    pub fn from_address(address: &str) -> Result<Self, MockClientError> {
        let addr = address
            .parse()
            .map_err(|_| MockClientError::InvalidAddressFormat(address.to_owned()))?;
        Ok(Self::new(addr))
    }

    pub fn new(addr: SocketAddr) -> Self {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let inner_client = rt.block_on(async {
            NodeClient::new(
                tonic::transport::Endpoint::from_shared(format!("http://{}", addr))
                    .unwrap()
                    .connect_lazy(),
            )
        });

        Self {
            addr,
            inner_client,
            rt,
        }
    }

    fn client(&self) -> NodeClient<Channel> {
        self.inner_client.clone()
    }

    pub fn wait_for_chain_length(&self, lenght: ChainLength, timeout: Duration) {
        let started = std::time::Instant::now();
        loop {
            if self.tip().chain_length() >= lenght {
                return;
            }
            if started.elapsed() > timeout {
                panic!("Timeout elapsed while waiting for chain to grow")
            }
            std::thread::sleep(CLIENT_RETRY_WAIT);
        }
    }

    pub fn handshake(&self, nonce: &[u8]) -> HandshakeResponse {
        let mut client = self.client();
        let request = tonic::Request::new(HandshakeRequest {
            nonce: nonce.to_vec(),
        });

        self.rt
            .block_on(client.handshake(request))
            .unwrap()
            .into_inner()
    }

    pub fn tip(&self) -> LibHeader {
        let mut client = self.client();
        let request = tonic::Request::new(TipRequest {});
        let response = self.rt.block_on(client.tip(request)).unwrap().into_inner();
        read_into(&response.block_header)
    }

    pub fn headers(&self, block_ids: &[Hash]) -> Result<Vec<LibHeader>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(BlockIds {
            ids: self.hashes_to_bin_vec(block_ids),
        });

        self.rt.block_on(async {
            let response = client
                .get_headers(request)
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
            self.headers_stream_to_vec(response.into_inner()).await
        })
    }

    fn hashes_to_bin_vec(&self, blocks_id: &[Hash]) -> Vec<Vec<u8>> {
        blocks_id
            .iter()
            .map(|x| self.hash_to_bin(x))
            .collect::<Vec<Vec<u8>>>()
    }

    fn hash_to_bin(&self, block_id: &Hash) -> Vec<u8> {
        block_id.as_ref().to_vec()
    }

    pub fn get_blocks(&self, blocks_id: &[Hash]) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(BlockIds {
            ids: self.hashes_to_bin_vec(blocks_id),
        });

        self.rt.block_on(async {
            let response = client
                .get_blocks(request)
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
            self.block_stream_to_vec(response.into_inner()).await
        })
    }

    pub fn get_genesis_block_hash(&self) -> Hash {
        Hash::from_str(&hex::encode(self.handshake(&[]).block0)).unwrap()
    }

    pub fn pull_blocks(&self, from: &[Hash], to: Hash) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(PullBlocksRequest {
            from: self.hashes_to_bin_vec(from),
            to: self.hash_to_bin(&to),
        });
        self.rt.block_on(async {
            let response = client
                .pull_blocks(request)
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
            self.block_stream_to_vec(response.into_inner()).await
        })
    }

    pub fn pull_blocks_to_tip(&self, from: Hash) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(PullBlocksToTipRequest {
            from: self.hashes_to_bin_vec(&[from]),
        });
        self.rt.block_on(async {
            let response = client
                .pull_blocks_to_tip(request)
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
            self.block_stream_to_vec(response.into_inner()).await
        })
    }

    async fn headers_stream_to_vec(
        &self,
        mut stream: tonic::codec::Streaming<Header>,
    ) -> Result<Vec<LibHeader>, MockClientError> {
        let mut headers: Vec<LibHeader> = Vec::new();
        while let Some(next_message) = stream
            .message()
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?
        {
            headers.push(read_into(&next_message.content));
        }
        Ok(headers)
    }

    async fn block_stream_to_vec(
        &self,
        mut stream: tonic::codec::Streaming<Block>,
    ) -> Result<Vec<LibBlock>, MockClientError> {
        let mut blocks: Vec<LibBlock> = Vec::new();
        while let Some(next_message) = stream
            .message()
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?
        {
            blocks.push(read_into(&next_message.content));
        }
        Ok(blocks)
    }

    async fn fragment_stream_to_vec(
        &self,
        mut stream: tonic::codec::Streaming<Fragment>,
    ) -> Result<Vec<LibFragment>, MockClientError> {
        let mut fragments: Vec<LibFragment> = Vec::new();
        while let Some(next_message) = stream
            .message()
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?
        {
            fragments.push(read_into(&next_message.content));
        }
        Ok(fragments)
    }

    pub fn pull_headers(&self, from: &[Hash], to: Hash) -> Result<Vec<LibHeader>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(PullHeadersRequest {
            from: self.hashes_to_bin_vec(from),
            to: self.hash_to_bin(&to),
        });
        self.rt.block_on(async {
            let response = client
                .pull_headers(request)
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
            let stream = response.into_inner();
            self.headers_stream_to_vec(stream).await
        })
    }

    pub fn upload_blocks(&self, lib_block: LibBlock) -> Result<(), MockClientError> {
        let mut client = self.client();

        let bytes = Vec::with_capacity(4096);
        let mut codec = Codec::new(bytes);
        lib_block.serialize(&mut codec).unwrap();
        let block = Block {
            content: codec.into_inner(),
        };

        let request = tonic::Request::new(stream::iter(vec![block]));
        self.rt
            .block_on(client.upload_blocks(request))
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        Ok(())
    }

    pub fn push_headers(&self, lib_header: LibHeader) -> Result<(), MockClientError> {
        let mut client = self.client();

        let header = Header {
            content: lib_header.serialize_as_vec().unwrap(),
        };

        let request = tonic::Request::new(stream::iter(vec![header]));
        self.rt
            .block_on(client.push_headers(request))
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        Ok(())
    }

    pub fn get_fragments(&self, ids: Vec<Hash>) -> Result<Vec<LibFragment>, MockClientError> {
        let mut client = self.client();
        let request = tonic::Request::new(FragmentIds {
            ids: self.hashes_to_bin_vec(&ids),
        });

        self.rt.block_on(async {
            let response = client
                .get_fragments(request)
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
            self.fragment_stream_to_vec(response.into_inner()).await
        })
    }
}

pub struct JormungandrWatchClient {
    addr: SocketAddr,
    inner_client: WatchClient<Channel>,
    rt: Arc<Runtime>,
}

impl Clone for JormungandrWatchClient {
    fn clone(&self) -> Self {
        JormungandrWatchClient::new(self.addr)
    }
}

impl fmt::Debug for JormungandrWatchClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JormungandrWatchClient")
            .field("host", &self.addr)
            .finish()
    }
}

pub type WatchTipNotifier = Arc<(Mutex<Result<LibHeader, tonic::Status>>, Condvar)>;

impl JormungandrWatchClient {
    pub fn from_address(address: &str) -> Result<Self, MockClientError> {
        let addr = address
            .parse()
            .map_err(|_| MockClientError::InvalidAddressFormat(address.to_owned()))?;
        Ok(Self::new(addr))
    }

    pub fn new(addr: SocketAddr) -> Self {
        let rt = Arc::new(Builder::new_current_thread().enable_all().build().unwrap());
        let inner_client = rt.block_on(async {
            WatchClient::new(
                tonic::transport::Endpoint::from_shared(format!("http://{}", addr))
                    .unwrap()
                    .connect_lazy(),
            )
        });

        Self {
            addr,
            inner_client,
            rt,
        }
    }

    fn client(&self) -> WatchClient<Channel> {
        self.inner_client.clone()
    }

    pub fn tip_subscription(&self) -> WatchTipNotifier {
        use futures::StreamExt;

        let rt = Arc::clone(&self.rt);
        let mut client = self.client();

        let (sender, receiver) = std::sync::mpsc::sync_channel(1);

        std::thread::spawn(move || {
            rt.block_on(async {
                let stream = client
                    .tip_subscription(TipSubscriptionRequest {})
                    .await
                    .unwrap()
                    .into_inner();

                let mut stream = stream.fuse();

                let mut cond_var: Option<WatchTipNotifier> = None;

                while let Some(header) = stream.next().await {
                    let new_tip: Result<LibHeader, tonic::Status> =
                        header.map(|header| read_into(&header.content));

                    if let Some(signal) = &cond_var {
                        let (guard, signal) = &**signal;
                        *guard.lock().unwrap() = new_tip;

                        signal.notify_one();
                    } else {
                        let pair = Arc::new((Mutex::new(new_tip), Condvar::new()));

                        sender.send(Arc::clone(&pair)).unwrap();

                        cond_var.replace(pair);
                    }
                }
            })
        });

        receiver.recv().unwrap()
    }

    pub fn block_subscription(
        &self,
        sender: std::sync::mpsc::Sender<Result<LibBlock, tonic::Status>>,
    ) {
        use futures::StreamExt;

        let rt = Arc::clone(&self.rt);
        let mut client = self.client();

        std::thread::spawn(move || {
            rt.block_on(async {
                let stream = client
                    .block_subscription(BlockSubscriptionRequest {})
                    .await
                    .unwrap()
                    .into_inner();

                let mut stream = stream.fuse();

                while let Some(block) = stream.next().await {
                    if sender
                        .send(block.map(|block| read_into(&block.content)))
                        .is_err()
                    {
                        break;
                    }
                }
            })
        });
    }

    pub fn sync_multiverse(&self, from: Vec<Hash>) -> Vec<LibBlock> {
        use futures::StreamExt;

        let mut blocks = vec![];

        let rt = Arc::clone(&self.rt);
        let mut client = self.client();

        rt.block_on(async {
            let stream = client
                .sync_multiverse(SyncMultiverseRequest {
                    from: from
                        .into_iter()
                        .map(|hash| hash.as_bytes().to_vec())
                        .collect(),
                })
                .await
                .unwrap()
                .into_inner();

            let mut stream = stream.fuse();

            while let Some(block) = stream.next().await {
                blocks.push(read_into(&block.unwrap().content))
            }
        });

        blocks
    }
}
