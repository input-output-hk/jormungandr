The following options are available in the log section:

- `level`: log messages minimum severity. If not configured anywhere, defaults to "info".
  Possible values: "off", "critical", "error", "warn", "info", "debug", "trace".
- `format`: log output format - `plain` or `json`.
- `output`: log output - `stdout`, `stderr`, `syslog` (Unix only),
  or `journald` (Linux with systemd only, must be enabled during compilation).
