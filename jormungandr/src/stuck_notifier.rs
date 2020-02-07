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
            let era = tip.epoch_leadership_schedule().era();

            let tip_date = tip.block_date();
            let tip_slot = era
                .from_era_to_slot(EpochPosition {
                    epoch: Epoch(tip_date.epoch),
                    slot: EpochSlotOffset(tip_date.slot_id),
                });
            let tip_time = tip.time_frame().slot_to_systemtime(tip_slot).ok_or_else(|| {
                error!(logger, "cannot convert the block tip date to system time");
            })?;

            let now = SystemTime::now();
            let system_current_slot = tip.time_frame().slot_at(&now);
            let system_current_blockdate = system_current_slot.and_then(|scs| era.from_slot_to_era(scs))
                .map(|ep| format!("{}", ep)).unwrap_or("date-computation-error".to_string());

            let header = tip.header();
            match now.duration_since(tip_time) {
                Ok(period_since_last_block) => {
                    if period_since_last_block > check_period {
                        warn!(
                            logger,
                            "blockchain is not moving up, system-date={}, the last tip {} was {} seconds ago",
                            system_current_blockdate, header.description(), period_since_last_block.as_secs()
                        );
                    }
                }
                Err(e) => {
                    // don't make the future fail because of this error. This can happen only
                    // if the tip has just been updated
                    error!(
                        logger,
                        "cannot compute the time passed since the last block: {}", e
                    );
                }
            }
            Ok(())
        })
}
