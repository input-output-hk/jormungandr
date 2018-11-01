use std::thread;
use std::time;
use rand;

use super::super::{clock, BlockchainR, TPoolR};

pub fn leadership_task(tpool: TPoolR, blockchain: BlockchainR, clock: clock::Clock) {
    loop {
        let d = clock.wait_next_slot();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();
        println!("slept for {:?} epoch {} slot {} next_slot {:?}", d, epoch.0, idx, next_time);
        let len = {
            let t = tpool.read().unwrap();
            (*t).content.len()
        };

        // TODO: check if this node is "elected" (by design or by stake) for this slot
        let elected = true;

        if elected {
            // create a new block to broadcast:
            // * get the transactions to put in the transactions
            // * mint the block
            // * sign it
            let latest_tip = {
                let b = blockchain.read().unwrap();
                b.get_tip()
            };

            println!("leadership create tpool={} transactions tip={}", len, latest_tip);

            // SIMULATING busy task: take between 1 to 21 seconds.
            {
                let v = 1u64 + (rand::random::<u64>() % 20);
                thread::sleep(time::Duration::from_secs(v))
            };
            // TODO: send it to block thread for appending/broadcasting
        }

    }
}
