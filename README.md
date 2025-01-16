### Near Protocol contracts for the Divvy Wealth client
[Divvy Wealth Litepaper](https://divvywealth.com)

#### Build deployable wasm
`env RUSTFLAGS='-Ctarget-cpu=mvp' cargo +nightly build -Zbuild-std=panic_abort,std --target=wasm32-unknown-unknown --release`



#### Run functional tests:
`cargo test`

#### Run integration tests:
`cargo test --test integration_tests`

#### Run clippy linter:
`cargo  clippy`

#### Related repositories
... add once private repos have been published.


