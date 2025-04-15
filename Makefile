check: test integration

test:
	cargo test

integration:
	cargo run -- examples/print_5.s 2>&1 \
		| grep 5 > /dev/null
	cargo run -- examples/add_5_7.s 2>&1 \
		| grep 12 > /dev/null
	(cargo run -- examples/fibonacci.s || true) 2>&1 \
		| grep 17711 > /dev/null
