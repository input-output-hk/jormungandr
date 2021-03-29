# Logging

The following options are available in the `log` section:

- `level`: log messages minimum severity. If not configured anywhere, defaults to `info`.
           Possible values: `off`, `critical`, `error`, `warn`, `info`, `debug`, `trace`

- `format`: Log output format, `plain` or `json`

- `output`: Log output destination (multiple destinations are supported). Possible values are:
  - `stdout`: standard output
  - `stderr`: standard error
  - `journald`: journald service (only available on Linux with systemd,
    (if jormungandr is built with the `systemd` feature)
  - `gelf`: Configuration fields for GELF (Graylog) network logging protocol
    (if jormungandr is built with the `gelf` feature):
    - `backend`: _hostname_:_port_ of a GELF server
    - `log_id`: identifier of the source of the log, for the `host` field in the messages
  - `file`: path to the log file

## Example

A single configurable backend is supported.

```yaml
log:
  - output: stdout
    level:  trace
    format: plain
```

```yaml
  - output:
    file: example.log
    level: info
    format: json
```
