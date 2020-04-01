
cd master
cargo build --release
cd ../slave
cargo build --release

cd ..
arm-none-eabi-objcopy -O binary target/thumbv6m-none-eabi/release/palantir-master target/thumbv6m-none-eabi/release/palantir-master.bin
arm-none-eabi-objcopy -O binary target/thumbv6m-none-eabi/release/palantir-slave target/thumbv6m-none-eabi/release/palantir-slave.bin
uf2conv-rs target/thumbv6m-none-eabi/release/palantir-master.bin -o flash-master.uf2
uf2conv-rs target/thumbv6m-none-eabi/release/palantir-slave.bin -o flash-slave.uf2