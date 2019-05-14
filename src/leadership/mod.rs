/*!

In a proof of work blockchain, the notion of time is bound to the time necessary
to perform the work. This time may vary a little, but its computation is rather
deterministic. In our proof of stake protocol the blocks are expected to be created
at every given `N` seconds. This value may vary depending of the configuration of
the blockchain. Also to know the schedule we need to be able to take snapshot of the
blockchain. These snapshots cover the length of a period of time known as `Epoch`.
There are `M` blocks in an `Epoch. And again this value may change depending of the
configuration of the blockchain.

# The Leadership runtime

The leadership is the task that manages the different aspect linked
to managing the different leaders of the node as well as notifying
the blockchain of new events such as new Epoch starting.


```text
                                          +-----------------------+
                         +--------------->+     Leader 1          |
                         |                +-----------------------+
                         |
+---------------+        |
|               |        |                +-----------------------+
|               +------------------------>+     Leader 2          |
|   Leadership  |        |                +-----------------------+
|   Process     |        |Spawn at
|   Manager     |        |every new
|               |        |Epoch
|               |        |                +-----------------------+
|               |        |                |   End of epoch        |
|               |        +--------------->+     Schedule          +--------+
|               |                         +-----------------------+        |
+--------+------+                                                          |
         ^                                                                 |
         |                                                                 |
         |                                                        Notify   |
         | Notify of a new                                        of a new |
         | Epoch with the                                         Epoch    |
         | appropriate                                                     |
         | values (ledger state                                            |
         | and parameters)                                                 |
         |                                                                 |
         |                         +---------------------+                 |
         |                         |                     |                 |
         |                         |                     |                 |
         |                         |   Blockchain        +<----------------+
         +-------------------------+   Process           |
                                   |   (Ledger states &  |
                                   |   block validation) |
                                   |                     |
                                   +---------------------+
```

Now instead of an active loop, we organize the leadership through a series
of events:

1. the first event is triggered at start up;
2. the [`Blockchain`] process will gather the necessary [`EpochParameters`]
   for the Leadership [`Process`];
3. the leadership [`Process`] will notify the different Leader [`Task`] a new
   `Epoch` has started. Each of these Leader [`Task`] prepare their own
   [`LeaderSchedule`] that will wake them every time it is their turn to create
   a block.
4. at the end of the `Epoch` the Leadership [`Process`] notify the [`Blockchain`]
   process a new epoch is expected (`2.`).

This is performed like this because the notion of blockchain time is linked to the
state of the blockchain at a given time and is only valid for a _given time_ too.

# The Leader [`Task`]

The leader [`Task`] is the task associated to a leader. It will receive the notification
from the Leadership [`Process`]: [`TaskParameters`]. This are the necessary information
required for the [`Task`] to create its [`LeaderSchedule`].

Every time the [`LeaderSchedule`] will wake the [`Task`] at every new event for the
task to create a new `Block` and then it will submit it to the [`Blockchain`] process.

[`Blockchain`]: #
[`Process`]: ./struct.Process.html
[`Task`]: ./struct.Task.html
[`EpochParameters`]: ./struct.EpochParameters.html
[`LeaderSchedule`]: ./struct.LeaderSchedule.html

*/

mod epoch_parameters;
pub mod leaderships;
mod process;
mod schedule;
mod task;

pub use self::leaderships::*;

pub use self::epoch_parameters::EpochParameters;
pub use self::process::{HandleEpochError, Process, ProcessError};
pub use self::schedule::{LeaderSchedule, ScheduledEvent};
pub use self::task::{Task, TaskParameters};
