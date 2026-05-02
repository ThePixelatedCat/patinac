$env:LLVM_PROFILE_FILE = 'cargo-test.profraw'; 
cargo test -p nameres --config build.incremental=false --config build.rustflags=['\"-Cinstrument-coverage\"']; 

grcov . --binary-path C:\Users\acfro\.cargo\target\debug\deps -s . -t html --ignore-not-existing --ignore '../*' --ignore "/*" --ignore "*/main.rs" --ignore "*/test.rs" -o target/coverage/;
# Remove-Item C:\Users\acfro\Documents\Programming\Projects\Languages\patina\cargo-test.profraw;
# Remove-Item C:\Users\acfro\Documents\Programming\Projects\Languages\patina\ast\cargo-test.profraw;
# Remove-Item C:\Users\acfro\Documents\Programming\Projects\Languages\patina\runtime\cargo-test.profraw;
Start-Process firefox file:///C:/Users/acfro/Documents/Programming/Languages/patina/target/coverage/html/index.html