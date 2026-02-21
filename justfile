mod nvs 'esp-nvs/justfile'

_default:
    @just --list

fix: nvs::fix partition_tool::fix
    cargo fmt

fmt-all: fmt
    just --unstable --format
    nixfmt devenv.nix
    nixfmt .nix/esp-nvs-partition-tool.nix

fmt: _nightly-fmt

lint: nvs::lint partition_tool::lint
    cargo fmt --check

test:
    cargo test --all
    cargo test --doc

update-changelog: nvs::update-changelog partition_tool::update-changelog

_nightly-fmt:
    devenv shell \
        --option languages.rust.version:string 2026-02-18 \
        --option languages.rust.channel:string nightly \
        cargo fmt --all

_nightly-fmt-check:
    devenv shell \
        --option languages.rust.version:string 2026-02-18 \
        --option languages.rust.channel:string nightly \
        cargo fmt --all --check
