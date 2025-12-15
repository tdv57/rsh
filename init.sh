> ~/.rshrc
echo PATH=$PATH > ~/.rshrc
cp ./target/release/rust_shell /bin/
exec /bin/rust_shell