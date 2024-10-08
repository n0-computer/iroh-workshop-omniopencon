# Code for the iroh workshop @ OmniOpenCon 2024

# Prerequisites

## Git

https://git-scm.com/downloads

## Rust

https://www.rust-lang.org/tools/install
```sh
curl https://sh.rustup.rs | sh
```
## VS Code

https://code.visualstudio.com/download

## Rust Analyzer plugin

https://rust-analyzer.github.io/
- rust-analyzer also works with Emacs and Vim
- you can install it from within vscode
![image](https://hackmd.io/_uploads/HJxLyV6ef0.png)

## Iroh CLI

```sh
cargo install iroh-cli`
```

## Nice to have

sendme and dumbpipe
```sh    
cargo install dumbpipe
cargo install sendme
```

A dns inspection tool like `dig` or `nslookup`

Alternatively use https://toolbox.googleapps.com/apps/dig/#TXT/

## Discord (to exchange tickets etc)

https://iroh.computer/discord

Use channel #Workshop

## Slides

https://tinyurl.com/3sc3ecx8

## Hackmd

https://hackmd.io/LI1EwvgYTvC7I92aKvVgYA?view


You can run each step using e.g.

```
cd pipe1
cargo run
```

You can copy each example as a starting point for a standalone
crate.

## Pipe

Simple pipe between two endpoints anywhere in the world.

Like netcat, but global

/pipe-diy just the project setup, DIY
/pipe1 minimal working version
/pipe2 use iroh DNS node discovery to get shorter tickets
/pipe3 use https://pkarr.org node discovery to get p2p discovery
/pipe4 add direct addresses to the published records

## Chat

Peer to peer group chat using iroh gossip protocol

/chat-diy just the project setup, DIY
/chat1 minimal working version, text protocol
/chat2 messages signed by the node id
/chat3 add encrypted direct messages

## Raw Chat

Same as above, but implemented using iroh-net and iroh-gossip instead of using
iroh.

## Links

- Discord: https://iroh.computer/discord
- Iroh docs: https://docs.rs/iroh
- Our blog: https://iroh.computer/blog
