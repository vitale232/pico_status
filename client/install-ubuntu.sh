#!/usr/bin/env bash

# simply moves the built binary to /usr/bin

if [[ $UID != 0 ]]; then
    echo "Please run this script with sudo:"
    echo "sudo $0 $*"
    exit 1
fi

rm -f /usr/bin/pico-client
cp ./target/release/pico-client /usr/bin/pico-client
