# Prometheus

## Prerequisites

To use Prometheus you need Jormungandr compiled with the `prometheus-metrics` feature enabled.

## Usage

To enable Prometheus endpoint you need to enable it in the configuration file:

```yaml
prometheus:
  enabled: true
```

Alternatively, you can use the `--prometheus-metrics` flag.

When enabled, the Prometheus endpoint is exposed as `http(s)://<API_ADDR>:<API_PORT>/prometheus`.
