#/usr/bin/sh

# currently not using this script, as cross compiling with my config requires
# we vendor openssl, which means it will get out of date.

PI_IP=192.168.1.138
TARGET=armv7-unknown-linux-gnueabihf

cargo build --target $TARGET --release

sshpass -f .pi_pass scp -r ./target/$TARGET/release/pico-client vitale232@$PI_IP:/home/vitale232/pico/pico-client
sshpass -f .pi_pass scp -r ./.env vitale232@$PI_IP:/home/vitale232/pico/.env
