fix:
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged --release --features=defmt
    cargo fmt

lint:
    cargo clippy --release --features=defmt -- -D warnings
    cargo fmt --check

test:
    cargo test

publish-dry-run:
    cargo publish --registry crates-io --dry-run

publish:
    cargo publish --registry crates-io

update-changelog:
    git-cliff --bump -o CHANGELOG.md

[working-directory: 'esp-nvs-lib/tests/assets/']
generate_test_nvs_bin:
    ../../target/release/nvs_part generate test_nvs_data.csv test_nvs_data.bin --size 0x4000
