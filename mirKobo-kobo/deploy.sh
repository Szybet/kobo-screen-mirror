#!/bin/bash

cargo install cross
RUSTFLAGS='-C link-arg=-s' ~/.cargo/bin/cross build --release --target armv7-unknown-linux-musleabihf

servername="root@192.168.2.2"
passwd="root"

#sshpass -p $passwd ssh $servername "bash -c \"ifsctl mnt rootfs rw\""
sshpass -p $passwd ssh $servername "bash -c \"rm /mir_kobo_kobo\""
sshpass -p $passwd scp target/armv7-unknown-linux-musleabihf/release/mir_kobo_kobo $servername:/
