It contains simple capnproto IPC messaging example.

It also contains `hole-punching` implementation via `ductr`,`autoNAT`,`relay` via `libp2p` networking stack.

For running kindly do - 

  ```
    // For running the relay node
    cargo run

    // For running client A 
    cargo run relay-server peer_multiaddr peer_id

    // For handshaking between relayed connection and client B
    cargo run relay-client --relay-addr libp2_relayed_p2p_address

    Example -:
    cargo run
    cargo run 12D3KooWLtDY9NJBJMLPag9HUtrLG4baugfWdFCbojY1hYjRJ8hU /ip4/23.111.156.126/tcp/40287
    cargo run --relay-addr /ip4/23.111.156.126/tcp/40287/p2p/12D3KooWLtDY9NJBJMLPag9HUtrLG4baugfWdFCbojY1hYjRJ8hU/p2p-circuit/p2p/12D3KooWLjgkAttKeHSnXNmrZpDf5ovAnNcC8aQTntyADVHcHREN



  ```
