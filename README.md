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

---

## iroh Example

An echo example built on the [`iroh`](https://crates.io/crates/iroh) networking
stack (QUIC, with relay + hole-punching and dialing by public key). Connectivity
uses the default n0 discovery service, so the listener's **Endpoint ID** is the
only thing the dialer needs.

Two components, started in order:

---

### 1. Listener

```bash
cd iroh-connect
cargo run --bin listen
```

Note the **Endpoint ID** printed on startup — needed for the dialer.

---

### 2. Dialer

Dials the listener by its Endpoint ID, sends a message, and prints the echoed reply.

```bash
cd iroh-connect
cargo run --bin connect -- <endpoint-id> "hello iroh"
```

---

## iroh over Tor

The same echo example, but routed over **Tor hidden services** using the
experimental [`iroh-tor-transport`](https://github.com/n0-computer/iroh-tor-transport)
custom transport. The peer's `.onion` address is **derived from its Endpoint ID**,
so you reach other nodes knowing only their Endpoint ID — no IP address, no NAT
hole-punching, and no public relay. This works even behind NATs that plain
hole-punching can't traverse (symmetric NATs), at the cost of Tor's latency.

### Prerequisite: a running Tor daemon with the control port enabled

```bash
tor --ControlPort 9051 --CookieAuthentication 0
```

Each side automatically publishes its own ephemeral hidden service on startup.

---

### 1. Listener

```bash
cd iroh-tor
cargo run --bin listen
```

Note the **Endpoint ID** printed on startup — needed for the dialer. To keep the
same identity (and onion address) across restarts, export the printed
`IROH_SECRET` before running.

---

### 2. Dialer

```bash
cd iroh-tor
cargo run --bin connect -- <endpoint-id> "hello over tor"
```

Establishing the Tor circuit and resolving the hidden service can take tens of
seconds on the first connect.


