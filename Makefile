check: test

debug:
	rust-lldb `make _latest_debug_target`

_latest_debug_target:
	@find target/debug/deps -type f -not -name "*.*" \
		| xargs -n1 stat -f "%m %N" -t "%s" \
		| sort \
		| tail -n1 \
		| cut -d' ' -f2

test:
	cargo test

demo:
	cargo run -- examples/*.s
