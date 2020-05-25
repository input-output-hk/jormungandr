use crate::mock::read_into;

use node::node_client::NodeClient;
use node::{
    node_server::{Node, NodeServer},
    Block, BlockEvent, BlockIds, Fragment, FragmentIds, Gossip, HandshakeRequest,
    HandshakeResponse, Header, PeersRequest, PeersResponse, PullBlocksToTipRequest,
    PullHeadersRequest, PushHeadersResponse, TipRequest, TipResponse, UploadBlocksResponse,
};

use chain_impl_mockchain::{
    block::Block as LibBlock, fragment::Fragment as LibFragment, header::Header as LibHeader,
    key::Hash,
};

use futures_util::stream;
use std::pin::Pin;
use tokio::sync::mpsc;

pub mod node {
    tonic::include_proto!("iohk.chain.node"); // The string specified here must match the proto package name
}

use chain_core::property::FromStr;
use chain_core::property::Serialize;

use thiserror::Error;

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
}

impl Clone for JormungandrClient {
    fn clone(&self) -> Self {
        JormungandrClient::new(&self.host, self.port)
    }
}

impl JormungandrClient {
    pub fn from_address(address: &str) -> Result<Self, MockClientError> {
        let elements: Vec<&str> = address.split("/").collect();

        let host = elements.get(2);
        let port = elements.get(4);

        if host.is_none() || port.is_none() {
            return Err(MockClientError::InvalidAddressFormat(address.to_owned()).into());
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
            port: port,
        }
    }

    fn address(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    pub async fn handshake(&self) -> HandshakeResponse {
        let mut client = NodeClient::connect(self.address()).await.unwrap();
        let request = tonic::Request::new(HandshakeRequest {});

        client.handshake(request).await.unwrap().into_inner()
    }

    pub async fn tip(&self) -> LibHeader {
        let mut client = NodeClient::connect(self.address()).await.unwrap();
        let request = tonic::Request::new(TipRequest {});
        let response = client.tip(request).await.unwrap().into_inner();
        read_into(&response.block_header)
    }

    pub async fn headers(&self, block_ids: &[Hash]) -> Result<Vec<LibHeader>, MockClientError> {
        let mut client = NodeClient::connect(self.address()).await.unwrap();

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
        block_id.as_ref().iter().cloned().collect()
    }

    pub async fn get_blocks(&self, blocks_id: &[Hash]) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = NodeClient::connect(self.address()).await.unwrap();

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
        Hash::from_str(&hex::encode(self.handshake().await.block0)).unwrap()
    }

    pub async fn pull_blocks_to_tip(&self, from: Hash) -> Result<Vec<LibBlock>, MockClientError> {
        let mut client = NodeClient::connect(self.address()).await.unwrap();

        let request = tonic::Request::new(PullBlocksToTipRequest {
            from: self.hashes_to_bin_vec(&vec![from]),
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
        loop {
            if let Some(next_message) = stream
                .message()
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?
            {
                headers.push(read_into(&next_message.content))
            }
            break;
        }
        Ok(headers)
    }

    async fn block_stream_to_vec(
        &self,
        mut stream: tonic::codec::Streaming<Block>,
    ) -> Result<Vec<LibBlock>, MockClientError> {
        let mut blocks: Vec<LibBlock> = Vec::new();
        loop {
            if let Some(next_message) = stream
                .message()
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?
            {
                blocks.push(read_into(&next_message.content))
            }
            break;
        }
        Ok(blocks)
    }

    async fn fragment_stream_to_vec(
        &self,
        mut stream: tonic::codec::Streaming<Fragment>,
    ) -> Result<Vec<LibFragment>, MockClientError> {
        let mut fragments: Vec<LibFragment> = Vec::new();
        loop {
            if let Some(next_message) = stream
                .message()
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?
            {
                fragments.push(read_into(&next_message.content))
            }
            break;
        }
        Ok(fragments)
    }

    pub async fn pull_headers(
        &self,
        from: &[Hash],
        to: Hash,
    ) -> Result<Vec<LibHeader>, MockClientError> {
        let mut client = NodeClient::connect(self.address()).await.unwrap();

        let mut request = tonic::Request::new(PullHeadersRequest {
            from: self.hashes_to_bin_vec(from),
            to: self.hash_to_bin(&to),
        });
        let response = client
            .pull_headers(request)
            .await
            .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?;
        let mut blocks: Vec<LibHeader> = Vec::new();
        let mut stream = response.into_inner();
        loop {
            if let Some(next_message) = stream
                .message()
                .await
                .map_err(|err| MockClientError::InvalidRequest(err.message().to_string()))?
            {
                blocks.push(read_into(&next_message.content))
            }
            break;
        }
        Ok(blocks)
    }

    pub async fn upload_blocks(&self, lib_block: LibBlock) -> Result<(), MockClientError> {
        let mut client = NodeClient::connect(self.address()).await.unwrap();

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
        let mut client = NodeClient::connect(self.address()).await.unwrap();

        let mut header = Header {
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
        let mut client = NodeClient::connect(self.address()).await.unwrap();
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
