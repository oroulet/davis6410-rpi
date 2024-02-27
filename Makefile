SHELL=/bin/bash

all:
	source env.sh; cargo build

check:
	source env.sh; cargo check

test:
	# running with one thread since we are sharing db between tests
	source env.sh; cargo test -- --nocapture --test-threads=1

run:
	cargo run --bin wind_service

emu:
	cargo run --bin wind_service -- --emulation
