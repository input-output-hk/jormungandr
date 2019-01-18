use blockcfg::cardano::{Block, Cardano};
use blockchain::BlockchainR;
use settings::network::{Connection, Peer};

use futures::{future, Future, Stream};
use tokio::{
    net::tcp::{ConnectFuture, TcpStream},
    runtime::current_thread,
};
use tower_grpc::{Request, Streaming};
use tower_h2::client;
use tower_util::MakeService;

use super::cardano as cardano_proto;
use super::iohk::jormungandr as gen;

fn deserialize_block(block: cardano_proto::Block) -> Result<Block, cbor_event::Error> {
    let mut de = cbor_event::de::Deserializer::from(
        std::io::Cursor::new(&block.content)
    );
    de.deserialize_complete()
}

pub fn bootstrap_from_peer(peer: Peer, blockchain: BlockchainR<Cardano>) {
    info!("connecting to bootstrap peer {}", peer.connection);

    let mut make_client = client::Connect::new(
        peer,
        Default::default(),
        current_thread::TaskExecutor::current(),
    );

    let bootstrap = make_client
        .make_service(())
        .map(move |conn| {
            use self::gen::client::Node;

            // TODO: add origin URL with add_origin middleware from tower-http

            Node::new(conn)
        })
        .map_err(|e| {
            error!("failed to connect to bootstrap peer: {:?}", e);
        })
        .and_then(|mut client| {
            let tip = blockchain.read().unwrap().get_tip();
            let req = cardano_proto::HeaderHashes {
                hashes: vec![Vec::from(&tip.as_hash_bytes()[..])],
            };
            client
                .stream_blocks_to_tip(Request::new(req))
                .map_err(|e| {
                    error!("StreamBlocksToTip request failed: {:?}", e);
                })
                .and_then(|response| bootstrap_to_tip(blockchain, response.into_inner()))
        });

    match current_thread::block_on_all(bootstrap) {
        Ok(()) => debug!("bootstrap complete"),
        Err(()) => {
            // All specific errors should be logged and mapped to () in
            // future/stream error handling combinators.
        }
    }
}

fn bootstrap_to_tip(
    blockchain: BlockchainR<Cardano>,
    stream: Streaming<cardano_proto::Block, tower_h2::RecvBody>,
) -> impl Future<Item = (), Error = ()> {
    stream
        .fold(blockchain, |blockchain, block| {
            let block = match deserialize_block(block) {
                Ok(block) => block,
                Err(e) => {
                    error!("received malformed block from the bootstrap node: {:?}", &e);
                    return future::err(tower_grpc::Error::Inner(()));
                }
            };
            debug!("received block from the bootstrap node: {:#?}", &block);
            blockchain
                .write()
                .unwrap()
                .handle_incoming_block(block.into());
            future::ok(blockchain)
        })
        .map(|_| ())
        .map_err(|e| {
            error!("bootstrap block streaming failed: {:?}", e);
        })
}

impl tokio_connect::Connect for Peer {
    type Connected = TcpStream;
    type Error = ::std::io::Error;
    type Future = ConnectFuture;

    fn connect(&self) -> Self::Future {
        match &self.connection {
            Connection::Tcp(ref addr) => TcpStream::connect(addr),
            #[cfg(unix)]
            Connection::Unix(_) => unimplemented!(),
        }
    }
}
