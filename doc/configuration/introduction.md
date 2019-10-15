This chapter covers the node documentation, necessary to have a working system. It covers
the network, logging and storage parameters.

The node configuration uses the [YAML](https://en.wikipedia.org/wiki/YAML) format.

This is an example of a configuration file:

```YAML
storage: "/tmp/storage"
log:
  level: debug
  format: json
p2p:
  trusted_peers:
    - address: "/ip4/104.24.28.11/tcp/8299"
      id: ed25519_pk1tyz2drx5kvsp49fk0gunkl8ktlhjc976zj5erqwavnpfmrexxxnscxln75
    - address: "/ip4/104.24.29.11/tcp/8299"
      id: ed25519_pk13j4eata8e567xwdqp6wjeu8wa7dsut3kj0u3tgulrsmyvveq9qxqeqr3kc
    - address: "/ip4/13.113.10.64/tcp/3000"
      id: ed25519_pk16pw2st5wgx4558c6temj8tzv0pqc37qqjpy53fstdyzwxaypveys3qcpfl
    - address: "/ip4/13.52.208.132/tcp/3000"
      id: ed25519_pk15ppd5xlg6tylamskqkxh4rzum26w9acph8gzg86w4dd9a88qpjms26g5q9
    - address: "/ip4/163.172.195.51/tcp/4444"
      id: ed25519_pk1qe94tdfytllgkt9x9zy8p30g6lffu67z8ys0syfllt5vjvrz9fjqfmaqu0
    - address: "/ip4/192.99.102.208/tcp/5555"
      id: ed25519_pk1py38g8s6x5kchhkze05r2uq7x740vem0jd6hlpztlerfsxqzsz8q5zutes
    - address: "/ip4/3.115.194.22/tcp/3000"
      id: ed25519_pk1npsal4j9p9nlfs0fsmfjyga9uqk5gcslyuvxy6pexxr0j34j83rsf98wl2
    - address: "/ip4/3.120.96.93/tcp/3000"
      id: ed25519_pk10gmg0zkxpuzkghxc39n3a646pdru6xc24rch987cgw7zq5pmytmszjdmvh
    - address: "/ip4/51.79.35.204/tcp/6666"
      id: ed25519_pk1py38g8s6x5kchhkze05r2uq7x740vem0jd6hlpztlerfsxqzsz8q5zutes
    - address: "/ip4/52.28.134.8/tcp/3000"
      id: ed25519_pk1unu66eej6h6uxv4j4e9crfarnm6jknmtx9eknvq5vzsqpq6a9vxqr78xrw
    - address: "/ip4/52.57.214.174/tcp/3000"
      id: ed25519_pk1v4cj0edgmp8f2m5gex85jglrs2ruvu4z7xgy8fvhr0ma2lmyhtyszxtejz
    - address: "/ip4/54.153.19.202/tcp/3000"
      id: ed25519_pk1j9nj2u0amlg28k27pw24hre0vtyp3ge0xhq6h9mxwqeur48u463s0crpfk
    - address: "/ip4/54.153.19.202/tcp/3000"
      id: ed25519_pk1j9nj2u0amlg28k27pw24hre0vtyp3ge0xhq6h9mxwqeur48u463s0crpfk
    - address: "/ip4/72.226.119.133/tcp/61111"
      id: ed25519_pk1upweqll5ujee3ptswqnnada384ted506v3f8ezds53svwcpa0crsm5cycs
    - address: "/ip4/91.121.85.221/tcp/3100"
      id: ed25519_pk1r0ek988zzgfggcf6anqyczgme6drk4763mk8yg0cm36whh9y447q7frnx6
    - address: "/ip4/94.11.44.127/tcp/1337"
      id: ed25519_pk14ky8xmwykje2e94dazxfry4006nsptlc2huusaa480dat05vmuaq03sugp
    - address: "/ip4/94.195.82.31/tcp/3100"
      id: ed25519_pk1l22vf7hgyngegv5hj584e7p7yzw5fwm5a6nm9krsasqjvqzqxt7q80hv0q      
  public_address: "/ip4/127.0.0.1/tcp/8080"
  private_id: ed25519_sk1n649x7zrmt38q6dtqkvj769wru4vpkfnrppw6d83qd7jh3uhux7qwhg8q3
  topics_of_interest:
    messages: low
    blocks: normal
```

