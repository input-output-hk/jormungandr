use crate::utils::task::TokioServiceInfo;
use chain_impl_mockchain::{block::Block, header::HeaderId};
use chain_network::data::Peer;
use futures::{channel::mpsc, prelude::*, stream};
use rand::RngCore;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Debug,
    mem::replace,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::{Mutex, RwLock, Semaphore};

pub enum BlockDownloaderInput<StBlocks> {
    IncomingBlockStream { peer: Peer, stream: StBlocks },
    AddPeer { peer: Peer },
    RemovePeer { peer: Peer },
}

pub enum BlockDownloaderOutput {
    RequestDownloadFromPeer {
        peer: Peer,
        block_ids: Vec<HeaderId>,
    },
}

/// This structure represents the state of a peer that we use to download new
/// blocks. The main goal of this is to ensure than we use a peer to download
/// only one block stream at a time.
struct BlockDownloadSource {
    state: Arc<RwLock<BlockDownloadSourceState>>,
}

/// The state of a peer.
enum BlockDownloadSourceState {
    /// No downloads are pending or in progress and this peer can be used to
    /// download blocks from it.
    Ready,
    /// Pending download.
    Waiting { block_ids: Vec<HeaderId> },
    /// The download is currently in progress.
    Busy,
}

#[derive(Debug)]
pub enum DownloadError<BlockSinkError: Error + Debug> {
    /// The current source state cannot be used to process the incoming blocks
    /// stream.
    WrongSourceState,
    /// En error occurred in the block sink.
    BlockSinkError(BlockSinkError),
}

/// This structure maintains the list of peers to download blocks from and
/// provides the logic for selecting those peers.
#[derive(Clone)]
struct BlockDownloadSourceManager {
    peers: Arc<RwLock<HashMap<Peer, BlockDownloadSource>>>,
    // This semaphore is used to block when there is no ready peers.
    available_peer_count_semaphore: Arc<Semaphore>,
}

/// The structure returned by `BlockDownloadSourceManager::select`. It
/// encapsulates a `BlockDownloadSource` instance. Upon a `drop()` call it
/// ensures that this source released back to the parent
/// `BlockDownloadSourceManager` instance.
struct ManagedBlockDownloadSource {
    peer: Peer,
    source: BlockDownloadSource,
    peers: Arc<RwLock<HashMap<Peer, BlockDownloadSource>>>,
    peer_count_semaphore: Arc<Semaphore>,
}

/// The top-level download control structure responsible for handling download
/// restarts in an event of download interruption.
#[derive(Clone, Default)]
struct BlockDownloadTaskManager {
    tasks: Arc<Mutex<HashMap<Peer, (mpsc::Sender<Block>, ManagedBlockDownloadSource)>>>,
}

/// Limits the number of blocks in the queue.
#[derive(Clone)]
struct BlockDownloadBackPressureHandler {
    trigger: usize,
    release: usize,
    current_value: Arc<AtomicUsize>,
    is_active: Arc<AtomicBool>,
    semaphore: Arc<Semaphore>,
}

#[derive(Debug)]
pub enum DownloadServiceError<BlockSinkError: Error + Debug> {
    BlockSinkDead,
    BlockSink(BlockSinkError),
}

#[derive(Debug)]
pub enum DownloadTaskError<OutputSinkError: Error + Debug> {
    OutputSink(OutputSinkError),
    DownloadError(DownloadError<mpsc::SendError>),
}

#[derive(Debug)]
pub enum PreDownloadTaskError<OutputSinkError: Error + Debug> {
    OutputSink(OutputSinkError),
    BlockStreamsSink(mpsc::SendError),
}

impl BlockDownloadSource {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(BlockDownloadSourceState::Ready)),
        }
    }

    /// Check if this source is ready to be used for the download.
    async fn is_ready(&self) -> bool {
        let state = self.state.read().await;
        if let BlockDownloadSourceState::Ready = *state {
            true
        } else {
            false
        }
    }

    /// Queue the list of block IDs for the download.
    ///
    /// # Returns
    ///
    /// `true` if the download was queued successfully and `false` otherwise.
    async fn start_download(&self, block_ids: Vec<HeaderId>) -> bool {
        let mut state = self.state.write().await;
        if let BlockDownloadSourceState::Ready = *state {
            *state = BlockDownloadSourceState::Waiting { block_ids };
            true
        } else {
            false
        }
    }

    /// Cancel download if it has not started yet.
    async fn cancel_download(&self) {
        let mut state = self.state.write().await;
        if let BlockDownloadSourceState::Waiting { .. } = *state {
            *state = BlockDownloadSourceState::Ready;
        }
    }

    /// Download blocks from `block_stream` to `block_sink`. This function will
    /// exit if the stream contains an unexpected block or just ended.
    ///
    /// # Returns
    ///
    /// The `Ok` variant is an `Option` and the number of downloaded blocks.
    /// `None` is returned if all scheduled blocks were successfully
    /// downloaded. Otherwise, the list of blocks that are still to be
    /// downloaded is returned.
    async fn process_download<St, Si>(
        &self,
        block_stream: St,
        mut block_sink: Si,
    ) -> Result<(Option<Vec<HeaderId>>, usize), DownloadError<Si::Error>>
    where
        St: Stream<Item = Block> + Unpin,
        Si: Sink<Block> + Unpin,
        Si::Error: Error,
    {
        // lock state only for the time of checking and changing it
        let mut block_ids = {
            let mut state = self.state.write().await;
            match *state {
                BlockDownloadSourceState::Waiting { .. } => {}
                _ => return Err(DownloadError::WrongSourceState),
            }
            if let BlockDownloadSourceState::Waiting { block_ids } =
                replace(&mut *state, BlockDownloadSourceState::Busy)
            {
                block_ids
            } else {
                unreachable!("already checked that the variant is Waiting");
            }
        };

        let expected_block_ids_stream = stream::iter(block_ids.iter().enumerate());
        let mut block_stream = block_stream.zip(expected_block_ids_stream);
        let mut blocks_processed = 0;
        while let Some((block, (i, expected_block_id))) = block_stream.next().await {
            if block.header.id() != *expected_block_id {
                break;
            }
            block_sink
                .send(block)
                .await
                .map_err(DownloadError::BlockSinkError)?;
            blocks_processed = i + 1;
        }

        let tail = block_ids.split_off(blocks_processed);
        let result = if tail.is_empty() {
            Ok(None)
        } else {
            Ok(Some(tail))
        };

        let mut state = self.state.write().await;
        *state = BlockDownloadSourceState::Ready;

        result.map(|maybe_tail| (maybe_tail, block_ids.len()))
    }
}

impl Clone for BlockDownloadSource {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl BlockDownloadSourceManager {
    fn new() -> Self {
        Self {
            peers: Default::default(),
            available_peer_count_semaphore: Arc::new(Semaphore::new(0)),
        }
    }

    /// Add a peer to download blocks from.
    async fn add_peer(&self, peer: Peer) {
        let mut peers = self.peers.write().await;
        peers.insert(peer, BlockDownloadSource::new());
        self.available_peer_count_semaphore.add_permits(1);
    }

    /// Remove a peer. If it is currently in use, the operations will be ran
    /// until completion.
    async fn remove_peer(&self, peer: &Peer) {
        let mut peers = self.peers.write().await;
        if let Some(downloader) = peers.remove(peer) {
            // if a peer is in `Ready` state then the semaphore is not acquired
            // by this peer and we need to decrease the number of permits here
            if downloader.is_ready().await {
                self.available_peer_count_semaphore.acquire().await.forget();
            }
        }
    }

    /// Select the peer to download blocks. This function locks if no peers are
    /// available to start the download.
    ///
    /// # Returns
    ///
    /// A peer identifier and a handle object to download blocks from it.
    async fn select<R>(&self, block_ids: Vec<HeaderId>, rng: &mut R) -> ManagedBlockDownloadSource
    where
        R: RngCore,
    {
        use futures::future::join_all;

        let peer_count_guard = self.available_peer_count_semaphore.acquire().await;
        let peers = self.peers.read().await;
        let ready_peers_flags = join_all(
            peers
                .iter()
                .map(|(_peer, downloader)| downloader.is_ready()),
        )
        .await;
        let n_ready_peers = ready_peers_flags.iter().filter(|x| **x).count();
        if n_ready_peers == 0 {
            unreachable!("the peer counting semaphore is checked before");
        }
        let selection = rng.next_u64() as usize % n_ready_peers;
        let (peer, downloader) = peers
            .iter()
            .zip(ready_peers_flags.into_iter())
            .filter_map(|(kv, is_ready)| if is_ready { Some(kv) } else { None })
            .nth(selection)
            .unwrap();
        downloader.start_download(block_ids).await;
        // decrease the number of permits by one until the download is finished
        peer_count_guard.forget();
        ManagedBlockDownloadSource {
            peer: peer.clone(),
            source: downloader.clone(),
            peers: self.peers.clone(),
            peer_count_semaphore: self.available_peer_count_semaphore.clone(),
        }
    }
}

impl ManagedBlockDownloadSource {
    fn peer(&self) -> &Peer {
        &self.peer
    }

    /// See `BlockDownloadSource::process_download`.
    async fn process_download<St, Si>(
        &self,
        block_stream: St,
        block_sink: Si,
    ) -> Result<(Option<Vec<HeaderId>>, usize), DownloadError<Si::Error>>
    where
        St: Stream<Item = Block> + Unpin,
        Si: Sink<Block> + Unpin,
        Si::Error: Error,
    {
        self.source.process_download(block_stream, block_sink).await
    }
}

impl Drop for ManagedBlockDownloadSource {
    fn drop(&mut self) {
        // A hack to perform Drop in an async context. We actually need to run
        // logic to release this resource back to the manager if it was not
        // removed from the manager before.
        let peers = self.peers.clone();
        let peer = self.peer.clone();
        let peer_count_semaphore = self.peer_count_semaphore.clone();
        let source = self.source.clone();
        tokio::spawn(async move {
            let peers = peers.read().await;
            // automatically increase the peer count if this peer was not
            // removed from the manager
            if peers.contains_key(&peer) {
                peer_count_semaphore.add_permits(1);
            }
            source.cancel_download().await;
        });
    }
}

impl BlockDownloadBackPressureHandler {
    /// # Arguments
    ///
    /// * `trigger` - the number of pending blocks after which `.add()` will
    ///   lock.
    /// * `release` - the number of pending blocks after which `.add()` will
    ///   be unlocked if it was locked.
    ///
    /// # Panics
    ///
    /// When `trigger` is less than or equal to `release`.
    fn new(trigger: usize, release: usize) -> Self {
        assert!(trigger > release, "`trigger` must be higher than `release`");
        Self {
            trigger,
            release,
            current_value: Arc::new(AtomicUsize::new(0)),
            is_active: Arc::new(AtomicBool::new(false)),
            semaphore: Arc::new(Semaphore::new(1)),
        }
    }

    /// Add unprocessed blocks. Note that this will lock when this handler is triggered.
    async fn add(&self, n: usize) {
        let guard = self.semaphore.acquire().await;
        let previous_value = self.current_value.fetch_add(n, Ordering::Relaxed);
        if previous_value + n >= self.trigger {
            self.is_active.store(true, Ordering::Relaxed);
            guard.forget();
        }
    }

    /// Reduce the number of unprocessed blocks.
    fn sub(&self, n: usize) {
        let previous_value = self.current_value.fetch_sub(n, Ordering::Relaxed);
        if previous_value - n > self.release {
            return;
        }
        if !self.is_active.fetch_and(false, Ordering::Relaxed) {
            return;
        }
        self.semaphore.add_permits(1);
    }
}

impl BlockDownloadTaskManager {
    fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new block download task. The task is not considered as
    /// finished until all blocks from that task are downloaded.
    ///
    /// # Returns
    ///
    /// The id of the peer used for download that is later used to identify
    /// this task and a stream of downloaded blocks.
    async fn new_task<R>(
        &self,
        block_ids: Vec<HeaderId>,
        manager: &BlockDownloadSourceManager,
        rng: &mut R,
    ) -> (Peer, impl Stream<Item = Block>)
    where
        R: RngCore,
    {
        let (block_sink, block_stream) = mpsc::channel(block_ids.len());
        let mut tasks = self.tasks.lock().await;
        let source = manager.select(block_ids, rng).await;
        let peer = source.peer().clone();
        tasks.insert(peer.clone(), (block_sink, source));
        (peer, block_stream)
    }

    /// The overall logic is the same as in
    /// `BlockDownloadSource::process_download`. In addition the user needs to
    /// provide the `peer` they obtained from `new_task` and the new `peer` in
    /// case when there are pending blocks to download.
    async fn process_download<St, R>(
        &self,
        peer: Peer,
        block_stream: St,
        manager: &BlockDownloadSourceManager,
        rng: &mut R,
    ) -> Result<
        (Option<(Peer, Vec<HeaderId>)>, usize),
        DownloadError<<mpsc::Sender<Block> as Sink<Block>>::Error>,
    >
    where
        St: Stream<Item = Block> + Unpin,
        R: RngCore,
    {
        let mut tasks = self.tasks.lock().await;
        let (block_sink, source) = if let Some(res) = tasks.remove(&peer) {
            res
        } else {
            return Ok((None, 0));
        };
        let (maybe_tail, n_blocks_downloaded) = source
            .process_download(block_stream, block_sink.clone())
            .await?;
        // A corner case: when we have only one source manager.select() will
        // block until a new peer appears.
        drop(source);
        let maybe_tail = if let Some(tail) = maybe_tail {
            let source = manager.select(tail.clone(), rng).await;
            let peer = source.peer().clone();
            tasks.insert(peer.clone(), (block_sink, source));
            Some((peer, tail))
        } else {
            None
        };
        Ok((maybe_tail, n_blocks_downloaded))
    }
}

pub async fn block_downloader_task<StInput, StIds, StBlocks, SiOutput, SiBlocks>(
    info: TokioServiceInfo,
    input: StInput,
    block_ids: StIds,
    output: SiOutput,
    blocks: SiBlocks,
    ids_chunk_size: usize,
    backpressure_trigger: usize,
    backpressure_release: usize,
) -> Result<(), DownloadServiceError<SiBlocks::Error>>
where
    StInput: Stream<Item = BlockDownloaderInput<StBlocks>> + Unpin,
    StIds: Stream<Item = HeaderId> + Unpin,
    StBlocks: Stream<Item = Block> + Unpin + Send + 'static,
    SiOutput: Sink<BlockDownloaderOutput> + Unpin + Clone + Send + 'static,
    SiBlocks: Sink<Block> + Unpin,
    SiBlocks::Error: Error + Debug,
    SiOutput::Error: Error + Debug,
{
    use rand::{rngs::SmallRng, thread_rng, SeedableRng};

    enum InputInner<StBlocks> {
        Input(BlockDownloaderInput<StBlocks>),
        BlockIdsChunk(Vec<HeaderId>),
    }

    let input = input.map(InputInner::Input).map(Ok);

    let block_ids_chunked = block_ids
        .ready_chunks(ids_chunk_size)
        .map(InputInner::BlockIdsChunk)
        .map(Ok);

    let input = futures::stream::select(input, block_ids_chunked);

    let (block_streams_sink, block_output_stream) = mpsc::unbounded();
    let block_output_future = block_output_stream
        .flatten()
        .map(Ok)
        .forward(blocks)
        .map(|result| match result {
            Ok(_) => Err(DownloadServiceError::BlockSinkDead),
            Err(err) => Err(DownloadServiceError::BlockSink(err)),
        });

    let mut input = futures::stream::select(input, block_output_future.into_stream());

    let backpressure_handler =
        BlockDownloadBackPressureHandler::new(backpressure_trigger, backpressure_release);
    let download_source_manager = BlockDownloadSourceManager::new();
    let download_task_manager = BlockDownloadTaskManager::new();

    while let Some(msg) = input.next().await {
        match msg? {
            InputInner::Input(BlockDownloaderInput::AddPeer { peer }) => {
                let download_source_manager = download_source_manager.clone();
                info.spawn("add_peer", async move {
                    download_source_manager.add_peer(peer).await
                });
            }

            InputInner::Input(BlockDownloaderInput::RemovePeer { peer }) => {
                let download_source_manager = download_source_manager.clone();
                info.spawn("remove_peer", async move {
                    download_source_manager.remove_peer(&peer).await
                });
            }

            InputInner::Input(BlockDownloaderInput::IncomingBlockStream { peer, stream }) => {
                let download_source_manager = download_source_manager.clone();
                let download_task_manager = download_task_manager.clone();
                let backpressure_handler = backpressure_handler.clone();
                let mut output = output.clone();
                info.spawn_fallible::<_, DownloadTaskError<SiOutput::Error>>(
                    "process_block_stream",
                    async move {
                        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
                        let (maybe_peer_and_tail, n_blocks_downloaded) = download_task_manager
                            .process_download(peer, stream, &download_source_manager, &mut rng)
                            .await
                            .map_err(DownloadTaskError::DownloadError)?;
                        backpressure_handler.sub(n_blocks_downloaded);
                        if let Some((peer, tail)) = maybe_peer_and_tail {
                            output
                                .send(BlockDownloaderOutput::RequestDownloadFromPeer {
                                    peer,
                                    block_ids: tail,
                                })
                                .await
                                .map_err(DownloadTaskError::OutputSink)?;
                        }
                        Ok(())
                    },
                );
            }

            InputInner::BlockIdsChunk(block_ids) => {
                let download_source_manager = download_source_manager.clone();
                let download_task_manager = download_task_manager.clone();
                let backpressure_handler = backpressure_handler.clone();
                let mut block_streams_sink = block_streams_sink.clone();
                let mut output = output.clone();
                info.spawn_fallible::<_, PreDownloadTaskError<SiOutput::Error>>(
                    "prepare_block_download",
                    async move {
                        let mut rng = SmallRng::from_rng(&mut thread_rng()).unwrap();
                        backpressure_handler.add(block_ids.len()).await;
                        let (peer, task_blocks_stream) = download_task_manager
                            .new_task(block_ids.clone(), &download_source_manager, &mut rng)
                            .await;
                        block_streams_sink
                            .send(task_blocks_stream)
                            .await
                            .map_err(PreDownloadTaskError::BlockStreamsSink)?;
                        output
                            .send(BlockDownloaderOutput::RequestDownloadFromPeer {
                                peer,
                                block_ids,
                            })
                            .await
                            .map_err(PreDownloadTaskError::OutputSink)?;
                        Ok(())
                    },
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_impl_mockchain::header::HeaderId;
    use quickcheck::{Arbitrary, Gen};
    use std::ops::Deref;
    use tokio::time::{timeout, Duration};

    // TODO replace TestBlocks with Vec<Block> when we are able to disable
    // block contents in testing. For now this is used to reduce test times to
    // some sensible numbers (minutes).
    const TEST_BLOCKS_MAX_SIZE: usize = 10;

    #[derive(Debug, Clone)]
    struct TestBlocks(Vec<Block>);

    impl Arbitrary for TestBlocks {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let size = g.next_u64() as usize % TEST_BLOCKS_MAX_SIZE;
            let mut blocks = Vec::with_capacity(size);
            for _ in 0..size {
                blocks.push(Arbitrary::arbitrary(g));
            }
            Self(blocks)
        }
    }

    impl Deref for TestBlocks {
        type Target = Vec<Block>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[tokio::test]
    async fn backpressure() {
        const TRIGGER: usize = 50;
        const RELEASE: usize = 10;
        let backpressure_handler = BlockDownloadBackPressureHandler::new(TRIGGER, RELEASE);
        backpressure_handler.add(TRIGGER).await;
        assert!(timeout(Duration::from_secs(1), backpressure_handler.add(1))
            .await
            .is_err());
        backpressure_handler.sub(10);
        assert!(timeout(Duration::from_secs(1), backpressure_handler.add(1))
            .await
            .is_err());
        backpressure_handler.sub(35);
        assert!(timeout(Duration::from_secs(1), backpressure_handler.add(1))
            .await
            .is_ok());
    }

    #[quickcheck_async::tokio]
    async fn block_downloader(blocks: TestBlocks) {
        if blocks.is_empty() {
            return;
        }

        let ids: Vec<_> = blocks.iter().map(|block| block.header.id()).collect();
        let block_stream = futures::stream::iter(blocks.iter().cloned());
        let (block_sink, downloaded_blocks) = mpsc::unbounded();
        let downloader = BlockDownloadSource::new();

        assert!(downloader.is_ready().await);
        assert!(downloader.start_download(ids.clone()).await);
        assert!(
            !downloader.start_download(ids.clone()).await,
            "start cannot be called again before the download is finished"
        );
        assert!(
            !downloader.is_ready().await,
            "downloader is in ready only when there is no pending downloader"
        );

        assert!(downloader
            .process_download(block_stream, block_sink)
            .await
            .unwrap()
            .0
            .is_none());

        let downloaded_blocks: Vec<_> = downloaded_blocks.collect().await;
        assert_eq!(*blocks, downloaded_blocks);

        assert!(downloader.is_ready().await);
        assert!(downloader.start_download(ids.clone()).await);
    }

    #[quickcheck_async::tokio]
    async fn block_downloader_empty_stream(blocks: TestBlocks) {
        if blocks.is_empty() {
            return;
        }

        let ids: Vec<_> = blocks.iter().map(|block| block.header.id()).collect();
        let block_stream: futures::stream::Empty<Block> = futures::stream::empty();
        let (block_sink, downloaded_blocks) = mpsc::unbounded();
        let downloader = BlockDownloadSource::new();

        assert!(downloader.is_ready().await);
        assert!(downloader.start_download(ids.clone()).await);
        assert!(
            !downloader.start_download(ids.clone()).await,
            "start cannot be called again before the download is finished"
        );
        assert!(
            !downloader.is_ready().await,
            "downloader is in ready only when there is no pending downloader"
        );

        assert_eq!(
            ids,
            downloader
                .process_download(block_stream, block_sink)
                .await
                .unwrap()
                .0
                .unwrap()
        );

        let expected_blocks: Vec<Block> = Vec::new();
        let downloaded_blocks: Vec<_> = downloaded_blocks.collect().await;
        assert_eq!(expected_blocks, downloaded_blocks);

        assert!(downloader.is_ready().await);
        assert!(downloader.start_download(ids.clone()).await);
    }

    #[quickcheck_async::tokio]
    async fn block_downloader_unexpected_blocks(
        blocks_expected1: TestBlocks,
        blocks_expected2: TestBlocks,
        blocks_unexpected: TestBlocks,
    ) {
        if blocks_expected1.is_empty()
            || blocks_expected2.is_empty()
            || blocks_unexpected.is_empty()
        {
            return;
        }

        let ids: Vec<_> = blocks_expected1
            .iter()
            .chain(blocks_expected2.iter())
            .map(|block| block.header.id())
            .collect();
        let block_stream = futures::stream::iter(
            blocks_expected1
                .iter()
                .chain(blocks_unexpected.iter())
                .cloned(),
        );
        let (block_sink, downloaded_blocks) = mpsc::unbounded();
        let downloader = BlockDownloadSource::new();

        assert!(downloader.is_ready().await);
        assert!(downloader.start_download(ids.clone()).await);
        assert!(
            !downloader.start_download(ids.clone()).await,
            "start cannot be called again before the download is finished"
        );
        assert!(
            !downloader.is_ready().await,
            "downloader is in ready only when there is no pending downloader"
        );

        let tail_expected: Vec<_> = blocks_expected2
            .iter()
            .map(|block| block.header.id())
            .collect();
        assert_eq!(
            tail_expected,
            downloader
                .process_download(block_stream, block_sink)
                .await
                .unwrap()
                .0
                .unwrap()
        );

        let downloaded_blocks: Vec<_> = downloaded_blocks.collect().await;
        assert_eq!(*blocks_expected1, downloaded_blocks);

        assert!(downloader.is_ready().await);
        assert!(downloader.start_download(ids.clone()).await);
    }

    #[tokio::test]
    async fn block_source_management() {
        let block_source_manager = BlockDownloadSourceManager::new();
        let mut rng = rand::thread_rng();
        let block_ids: Vec<HeaderId> = Vec::new();

        assert!(timeout(
            Duration::from_secs(1),
            block_source_manager.select(block_ids.clone(), &mut rng)
        )
        .await
        .is_err());

        let addr: std::net::SocketAddr = "127.0.0.1:4000".parse().unwrap();
        let peer = Peer::from(Peer::from(addr));

        block_source_manager.add_peer(peer.clone()).await;

        let block_source = timeout(
            Duration::from_secs(1),
            block_source_manager.select(block_ids.clone(), &mut rng),
        )
        .await
        .unwrap();

        assert!(timeout(
            Duration::from_secs(1),
            block_source_manager.select(block_ids.clone(), &mut rng)
        )
        .await
        .is_err());

        std::mem::drop(block_source);

        let block_source = timeout(
            Duration::from_secs(1),
            block_source_manager.select(block_ids.clone(), &mut rng),
        )
        .await
        .unwrap();

        block_source_manager.remove_peer(&peer).await;

        std::mem::drop(block_source);

        assert!(timeout(
            Duration::from_secs(1),
            block_source_manager.select(block_ids.clone(), &mut rng)
        )
        .await
        .is_err());
    }

    #[quickcheck_async::tokio]
    async fn block_download_manager(blocks1: TestBlocks, blocks2: TestBlocks) {
        if blocks1.is_empty() || blocks2.is_empty() {
            return;
        }

        let mut rng = rand::thread_rng();

        let ids: Vec<_> = blocks1
            .iter()
            .chain(blocks2.iter())
            .map(|block| block.header.id())
            .collect();
        let block_download_manager = BlockDownloadSourceManager::new();
        let block_download_task_manager = BlockDownloadTaskManager::new();
        let addr: std::net::SocketAddr = "127.0.0.1:4000".parse().unwrap();
        let peer = Peer::from(Peer::from(addr));

        block_download_manager.add_peer(peer).await;

        let (peer, downloaded_blocks) = block_download_task_manager
            .new_task(ids, &block_download_manager, &mut rng)
            .await;

        let block_stream = futures::stream::iter(blocks1.iter().cloned());
        let (peer, _ids) = block_download_task_manager
            .process_download(peer, block_stream, &block_download_manager, &mut rng)
            .await
            .unwrap()
            .0
            .unwrap();

        let block_stream = futures::stream::iter(blocks2.iter().cloned());
        assert!(block_download_task_manager
            .process_download(peer, block_stream, &block_download_manager, &mut rng,)
            .await
            .unwrap()
            .0
            .is_none());

        let expected_blocks: Vec<_> = blocks1.iter().chain(blocks2.iter()).cloned().collect();
        let actual_blocks: Vec<_> = downloaded_blocks.collect().await;
        assert_eq!(expected_blocks, actual_blocks);
    }
}
