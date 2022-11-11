cargo +nightly build -Z build-std=std,panic_abort --target aarch64-apple-darwin --release &&\
rm /usr/local/bin/ash && ln target/aarch64-apple-darwin/release/ash /usr/local/bin/ash