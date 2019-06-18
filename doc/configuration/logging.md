The following options are available in the logger section:

- *verbosity*: 
  - 0: warning
  - 1: info
  - 2: debug
  - 3 and above: trace
- *format*: log output format - plain or json.
- *output*: log output - stderr, syslog (unix only) or journald (linux with systemd only, must be enabled during compilation)