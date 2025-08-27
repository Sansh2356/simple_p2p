It contains simple capnproto IPC messaging example.

It also contains `hole-punching` implementation via `ductr`,`autoNAT`,`relay` via `libp2p` networking stack.

For running kindly do -

  ```
    #For running the relay node
    cargo run

    #For running client A
    cargo run relay-server --peer-addr

    #For handshaking between relayed connection and client B
    cargo run relay-client --libp2_relayed_p2p_address
  ```
