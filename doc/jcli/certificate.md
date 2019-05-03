# certificate

Tooling for offline transaction creation

## Builder

Builds a signed certificate.

The process can be split into steps on the first step certificate
is created.
```
jcli certificate new stake-pool-registration \
  --vrf-key <vrf-public-key> --kes-key <kes-public-key> \
  [--owner <owner-public-key>] \
  --serial <node-serial> \
  <output-file>
```

if output-file is omited result will be written to stdout. Once
certificate is ready you must sign it with the private keys of
all the owners:


```
jcli certificate sign <key> <input-file> <output-file>
```

