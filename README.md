# Code for the iroh workshop @ JOnTheBeach 2024

## Hackmd
https://hackmd.io/@GMgEoX9mQWKh09CQ-JktgA/BkwrZ6lfC
https://tinyurl.com/2rmf4a8x

You can run each step using e.g.

```
cargo run -p pipe1
```

You can copy each example as a starting point for a standalone
crate.

## Pipe

Simple pipe between two endpoints anywhere in the world.

Like netcat, but global

/pipe_diy just the project setup, DIY
/pipe1 minimal working version
/pipe2 use iroh DNS discovery to get shorter tickets
/pipe3 use https://pkarr.org discovery to get p2p discovery
/pipe4 add direct addresses to the published records

## Chat

Peer to peer group chat using the iroh gossip protocol

/chat_diy just the project setup, DIY
/chat1 minimal working version, text protocol
/chat2 messages signed by the node id
/chat3 add encrypted direct messages
/chat4 add unique message id to prevent deduplication
