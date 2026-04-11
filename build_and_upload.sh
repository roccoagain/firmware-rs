cargo build --release
cargo objcopy --release -- -O ihex target/release/firmware-rs.hex
tycmd upload target/release/firmware-rs.hex
tycmd monitor
