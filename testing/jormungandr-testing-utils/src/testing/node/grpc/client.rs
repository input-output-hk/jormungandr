use crate::testing::node::grpc::read_into;

use super::proto::{
    node_client::NodeClient, Block, BlockIds, Fragment, FragmentIds, HandshakeRequest,
    HandshakeResponse, Header, PullBlocksRequest, PullBlocksToTipRequest, PullHeadersRequest,
    TipRequest,
};

use chain_impl_mockchain::{
    block::Block as LibBlock, fragment::Fragment as LibFragment, header::ChainLength,
    header::Header as LibHeader, key::Hash,
};
use futures::stream;
use futures::FutureExt;
use std::fmt;
use std::time::Duration;

use chain_core::property::FromStr;
use chain_core::property::Serialize;
use tonic::transport::Channel;

use thiserror::Error;

const CLIENT_RETRY_WAIT: Duration = Duration::from_millis(500);

#[derive(Error, Debug, PartialEq)]
pub enum MockClientError {
    #[error("request failed with message {0}")]
    InvalidRequest(String),
    #[error(
        "could not parse address '{0}'. HINT: accepted format example: /ip4/127.0.0.1/tcp/9000"
    )]
    InvalidAddressFormat(String),
}

pub struct JormungandrClient {
    host: String,
    port: u16,
    inner_client: NodeClient<Channel>,
}

impl Clone for JormungandrClient {
    fn clone(&self) -> Self {
        JormungandrClient::new(&self.host, self.port)
    }
}

impl fmt::Debug for JormungandrClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JormungandrClient")
            .field("host", &self.host)
            .field("port", &self.port)
            .finish()
    }
}

impl JormungandrClient {
    pub fn from_address(address: &str) -> Result<Self, MockClientError> {
        let elements: Vec<&str> = address.split('/').collect();

        let host = elements.get(2);
        let port = elements.get(4);

        if host.is_none() || port.is_none() {
            return Err(MockClientError::InvalidAddressFormat(address.to_owned()));
        }

        let port: u16 = port
            .unwrap()
            .parse()
            .map_err(|_err| MockClientError::InvalidAddressFormat(address.to_owned()))?;
        Ok(Self::new(host.unwrap(), port))
    }

    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_owned(),
            port,
            inner_client: NodeClient::new(
                tonic::transport::Endpoint::from_shared(format!("http://{}:{}", host, port))
                    .unwrap()
                    .connect_lazy()
                    .unwrap(),
            ),
        }
    }

    fn client(&self) -> NodeClient<Channel> {
        self.inner_client.clone()
    }

    pub async fn wait_for_chain_length(&self, lenght: ChainLength, timeout: Duration) {
        futures::select! {
            _ = async {
                while self.tip().await.chain_length() < lenght {
                    tokio::time::sleep(CLIENT_RETRY_WAIT).await;
                }
            }.fuse() => {},
            _ = tokio::time::sleep(timeout).fuse() => panic!("Timeout elapsed while waiting for chain to grow"),
        }
    }

    pub async fn handshake(&self, nonce: &[u8]) -> HandshakeResponse {
        let mut client = self.client();
        let request = tonic::Request::new(HandshakeRequest {
            nonce: nonce.to_vec(),
        });

        client.handshake(request).await.unwrap().into_inner()
    }

    pub async fn tip(&self) -> LibHeader {
        let mut client = self.client();
        let request = tonic::Request::new(TipRequest {});
        let response = client.tip(request).await.unwrap().into_inner();
        read_into(&response.block_header)
    }

    pub async fn headers(&self, block_ids: &[Hash]) -> Result<Vec<LibHeader>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(BlockIds {
            ids: self.hashes_to_bin_vec(block_ids),
        });

        let response = client
            .get_headers(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        self.headers_stream_to_vec(response.into_inner()).await
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

    pub async fn get_blocks(&self, blocks_id: &[Hash]) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(BlockIds {
            ids: self.hashes_to_bin_vec(blocks_id),
        });

        let response = client
            .get_blocks(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        self.block_stream_to_vec(response.into_inner()).await
    }

    pub async fn get_genesis_block_hash(&self) -> Hash {
        Hash::from_str(&hex::encode(self.handshake(&[]).await.block0)).unwrap()
    }

    pub async fn pull_blocks(
        &self,
        from: &[Hash],
        to: Hash,
    ) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(PullBlocksRequest {
            from: self.hashes_to_bin_vec(from),
            to: self.hash_to_bin(&to),
        });
        let response = client
            .pull_blocks(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        self.block_stream_to_vec(response.into_inner()).await
    }

    pub async fn pull_blocks_to_tip(&self, from: Hash) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(PullBlocksToTipRequest {
            from: self.hashes_to_bin_vec(&[from]),
        });
        let response = client
            .pull_blocks_to_tip(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        self.block_stream_to_vec(response.into_inner()).await
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

    pub async fn pull_headers(
        &self,
        from: &[Hash],
        to: Hash,
    ) -> Result<Vec<LibHeader>, MockClientError> {
        let mut client = self.client();

        let request = tonic::Request::new(PullHeadersRequest {
            from: self.hashes_to_bin_vec(from),
            to: self.hash_to_bin(&to),
        });
        let response = client
            .pull_headers(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        let stream = response.into_inner();
        self.headers_stream_to_vec(stream).await
    }

    pub async fn upload_blocks(&self, lib_block: LibBlock) -> Result<(), MockClientError> {
        let mut client = self.client();

        let mut bytes = Vec::with_capacity(4096);
        lib_block.serialize(&mut bytes).unwrap();
        let block = Block { content: bytes };

        let request = tonic::Request::new(stream::iter(vec![block]));
        client
            .upload_blocks(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        Ok(())
    }

    pub async fn push_headers(&self, lib_header: LibHeader) -> Result<(), MockClientError> {
        let mut client = self.client();

        let header = Header {
            content: lib_header.serialize_as_vec().unwrap(),
        };

        let request = tonic::Request::new(stream::iter(vec![header]));
        client
            .push_headers(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        Ok(())
    }

    pub async fn get_fragments(&self, ids: Vec<Hash>) -> Result<Vec<LibFragment>, MockClientError> {
        let mut client = self.client();
        let request = tonic::Request::new(FragmentIds {
            ids: self.hashes_to_bin_vec(&ids),
        });

        let response = client
            .get_fragments(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        self.fragment_stream_to_vec(response.into_inner()).await
    }
}
