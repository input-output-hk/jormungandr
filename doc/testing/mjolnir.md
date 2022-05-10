# Mjolnir

Mjolnir is a load cli & api project which operates on jormungandr node.

## Build & Install

In order to build mjolnir in main project folder run:
```
cd testing/mjolnir
cargo build
cargo install --path . --force
```

## Quick Start

### CLI

Mjolnir can be used as a cli. It is capable of putting various load on jormungandr node.
It has couple of different load types:

* explorer    - Explorer load
* fragment    - Fragment load
* passive     - Passive Nodes bootstrap
* rest        - Rest load

Simplest load configuration is to use rest load with below parameters:

```
Rest load

USAGE:
    mjolnir.exe rest [FLAGS] [OPTIONS] --duration <duration> --endpoint <endpoint>

FLAGS:
    -h, --help       Prints help information
    -m, --measure    Prints post load measurements
    -V, --version    Prints version information

OPTIONS:
    -c, --count <count>                            Number of threads [default: 3]
        --delay <delay>                            Amount of delay [milliseconds] between sync attempts [default: 50]
    -d, --duration <duration>                      Amount of delay [seconds] between sync attempts
    -e, --endpoint <endpoint>                      Address in format: http://127.0.0.1:8002/api/
    -b, --progress-bar-mode <progress-bar-mode>    Show progress bar [default: Monitor]
```

### API

Mjolnir main purpose is to serve load api:

```
use jortestkit::load::{self, ConfigurationBuilder as LoadConfigurationBuilder, Monitor};
use std::time::Duration;

    //node initialization
    let mut jormungandr = ...

    let rest_client = jormungandr.rest();

    // create request generator for rest calls
    let request = mjolnir::generators::RestRequestGen::new(rest_client);

    // duration based load run (40 seconds)
    let config = LoadConfigurationBuilder::duration(Duration::from_secs(40))
        // with 5 threads
        .thread_no(5)
        // with delay between each request 0.01 s
        .step_delay(Duration::from_millis(10))
        // with monitor thread monitor status of load run each 0.1 s
        .monitor(Monitor::Progress(100))
        // with status printer which prints out status of load run each 1 s
        .status_pace(Duration::from_secs(1_000))
        .build();

    // initialize load in sync manner (duration of each request is calculated by time difference between receiving response and sending request )
    let stats = load::start_sync(request, config, "Jormungandr rest load test");

    // finally some way to assert expected correctness, like percentage of successful requests
    assert!((stats.calculate_passrate() as u32) > 95);
```

### full list of available commands

Full list of commands is available on `mjolnir --help` command.

```
mjolnir 0.1.0
Jormungandr Load CLI toolkit

USAGE:
    mjolnir.exe [FLAGS] [SUBCOMMAND]

FLAGS:
        --full-version      display full version details (software version, source version, targets and compiler used)
    -h, --help              Prints help information
        --source-version    display the sources version, allowing to check the source's hash used to compile this
                            executable. this option is useful for scripting retrieving the logs of the version of this
                            application
    -V, --version           Prints version information

SUBCOMMANDS:
    explorer    Explorer load
    fragment    Fragment load
    help        Prints this message or the help of the given subcommand(s)
    passive     Passive Nodes bootstrap
    rest        Rest load
```
