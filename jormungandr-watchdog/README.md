# Jormungandr Watchdog

Core functionality for the update of Jörmungandr to run as a micro-service
architecture.

## How to use

Add the following in your `Cargo.toml` file:

```toml
[dependencies]
jormungandr-watchdog = "0.1"
```

See `examples/stdin_echo.rs`

## Development

The project is still very much a work in progress. It is now starting to be
usable and the APIs are likely to be changed often in order to improve usability
and stability.

Here is the list of missing items still being looked at:

- [ ] have the logging support as a service
    - [ ] configuration from the CLI and the config file
    - [ ] add gelf support
    - [ ] opentelemetry support
- [ ] notify on setting changes for a given service
- [ ] notify of a state update from a given service
- [ ] allow a service to access the watchdog controller in order to provide
      application control from a REST API for example

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.