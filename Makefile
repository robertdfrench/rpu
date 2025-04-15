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
	cargo run -- examples/nophop.s 2>&1 \
		| grep 20 > /dev/null
	cargo run -- examples/countdown.s 2>&1 \
		| tr '\n' ' ' \
		| grep '5 4 3 2 1 0' > /dev/null
