use crate::{blockchain, utils::task::TokioServiceInfo};
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    Epoch,
};
use futures::prelude::*;
use std::time::{Duration, SystemTime};
use tokio::timer::Interval;

pub fn check_last_block_time(
    service_info: TokioServiceInfo,
    blockchain_tip: blockchain::Tip,
    check_interval: Duration,
) -> impl Future<Item = (), Error = ()> {
    let logger = service_info.logger().clone();
    let err_logger = logger.clone();

    // those are different values, because check_interval can be big
    // (30 minutes) and the notification may remain unseen
    let check_period = check_interval;
    let notification_period = Duration::from_secs(60);

    Interval::new_interval(notification_period)
        .map_err(move |e| error!(err_logger, "timer error: {}", e))
        .and_then(move |_| blockchain_tip.get_ref())
        .for_each(move |tip| {
            let tip_date = tip.block_date();
            let slot = tip
                .epoch_leadership_schedule()
                .era()
                .from_era_to_slot(EpochPosition {
                    epoch: Epoch(tip_date.epoch),
                    slot: EpochSlotOffset(tip_date.slot_id),
                });
            let tip_time = tip.time_frame().slot_to_systemtime(slot).ok_or_else(|| {
                error!(logger, "cannot convert the block tip date to system time");
            })?;
            let period_since_last_block =
                SystemTime::now().duration_since(tip_time).map_err(|e| {
                    error!(
                        logger,
                        "cannot compute the time passed since the last block: {}", e
                    );
                })?;
            if period_since_last_block > check_period {
                warn!(
                    logger,
                    "blockchain is not moving up, the last block was {} seconds ago",
                    period_since_last_block.as_secs()
                );
            }
            Ok(())
        })
}
