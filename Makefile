help: #: Display this help menu
	@echo "USAGE:\n"
	@cat $(MAKEFILE_LIST) \
		| grep -v 'awk' \
		| awk -F':' '/#:/ { OFS=":"; print " ",$$1,$$3 }' \
		| column -s':' -t \
		| sort


check: test #: Run all tests

test: #: Just run cargo-based tests
	cargo test

build: #: Build an optimized binary
	cargo build --release
