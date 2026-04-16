[![codecov](https://codecov.io/gh/Sansh2356/simple_p2p/graph/badge.svg?token=RPRHMU6TT1)](https://codecov.io/gh/Sansh2356/simple_p2p)

# simple_p2p

Hole-punching implementation using DCUtR, AutoNAT, and relay via the `libp2p` networking stack.

## Setup

Three components must be started in order:

---

### 1. Relay Server over a publicly accesible IP

```bash
cd relay
cargo run
```

Note the **PeerID** and **port** printed on startup — needed for the next two steps.

---

### 2. Listener (hole-punching)

```bash
cd hole-punching
cargo run -- --mode listen \
  --relay-addr /ip4/<relay-ip>/tcp/<relay-port>/p2p/<relay-peer-id>
```

Note the **Local PeerID** printed on startup — needed for the dialer.

---

### 3. Dialer (hole-punching)

```bash
cd hole-punching
cargo run -- --mode dial \
  --relay-addr /ip4/<relay-ip>/tcp/<relay-port>/p2p/<relay-peer-id> \
  --remote-peer-id <listener-peer-id>
```

---

## Simple Relay Setup

Three components must be started in order. No hole-punching — traffic is always forwarded through the relay.

---

### 1. Relay Server over a publicly accessible IP

```bash
cd relay
cargo run
```

Note the **PeerID** and **port** printed on startup.

---

### 2. Destination Client (behind NAT)

Registers a reservation with the relay so it becomes reachable via a circuit address.

```bash
cd relay_test_client_destination
cargo run -- <relay-peer-id> /ip4/<relay-ip>/tcp/<relay-port>
```

Note the **Local PeerID** printed on startup — needed for the source client.

---

### 3. Source Client (behind NAT)

Dials the destination through the relay circuit.

```bash
cd relay_test_client_source
cargo run -- --relay-addr /ip4/<relay-ip>/tcp/<relay-port>/p2p/<relay-peer-id>/p2p-circuit/p2p/<destination-peer-id>
```


