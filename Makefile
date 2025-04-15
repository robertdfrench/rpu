check: test

test:
	cargo test

demo:
	cargo run -- examples/*.s
