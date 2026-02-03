$env:LLVM_PROFILE_FILE='cargo-test.profraw'; 
cargo test --config build.incremental=false --config build.rustflags=['\"-Cinstrument-coverage\"'];

grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" --ignore "*/main.rs" -o target/coverage/;
Remove-Item C:\Users\acfro\Documents\Programming\Projects\Languages\patina\cargo-test.profraw;
Remove-Item C:\Users\acfro\Documents\Programming\Projects\Languages\patina\compiler\cargo-test.profraw;
Remove-Item C:\Users\acfro\Documents\Programming\Projects\Languages\patina\runtime\cargo-test.profraw;
Start-Process firefox file:///C:/Users/acfro/Documents/Programming/Projects/Languages/patina/target/coverage/html/index.html