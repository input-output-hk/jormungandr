use crate::blockchain;
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    Epoch,
};
use std::time::{Duration, SystemTime};
use tokio::time::interval;

pub async fn check_last_block_time(blockchain_tip: blockchain::Tip, check_interval: Duration) {
    // those are different values, because check_interval can be big
    // (30 minutes) and the notification may remain unseen
    let check_period = check_interval;
    let notification_period = Duration::from_secs(60);

    let mut interval = interval(notification_period);

    loop {
        interval.tick().await;
        let tip = blockchain_tip.get_ref().await;
        let era = tip.epoch_leadership_schedule().era();

        let now = SystemTime::now();

        let tip_date = tip.block_date();
        let tip_slot = era.from_era_to_slot(EpochPosition {
            epoch: Epoch(tip_date.epoch),
            slot: EpochSlotOffset(tip_date.slot_id),
        });
        let tip_time = if let Some(tip_time) = tip.time_frame().slot_to_systemtime(tip_slot) {
            tip_time
        } else {
            tracing::error!("cannot convert the block tip date to system time");
            break;
        };

        let system_current_slot = tip.time_frame().slot_at(&now);
        let system_current_blockdate = system_current_slot
            .and_then(|scs| era.from_slot_to_era(scs))
            .map(|ep| format!("{}", ep))
            .unwrap_or_else(|| "date-computation-error".to_string());

        let header = tip.header();
        match now.duration_since(tip_time) {
            Ok(period_since_last_block) => {
                if period_since_last_block > check_period {
                    tracing::warn!(
                            "blockchain is not moving up, system-date={}, the last tip {} was {} seconds ago",
                            system_current_blockdate, header.description(), period_since_last_block.as_secs()
                        );
                }
            }
            Err(e) => {
                // don't make the future fail because of this error. This can happen only
                // if the tip has just been updated
                tracing::error!("cannot compute the time passed since the last block: {}", e);
            }
        }
    }
}
