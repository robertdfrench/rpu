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

release: check build
	make _tarball RPU_VERSION=`git rev-parse HEAD`

build: #: Build an optimized binary
	cargo build --release

_tarball:
	mkdir -p build
	mkdir -p build/rpu-$(RPU_VERSION)
	mkdir -p build/rpu-$(RPU_VERSION)/examples
	cp release-template/* build/rpu-$(RPU_VERSION)
	cp rpu.6 build/rpu-$(RPU_VERSION)
	cp examples/* build/rpu-$(RPU_VERSION)/examples
	cp target/release/rpu build/rpu-$(RPU_VERSION)
	tar -czf build/rpu-$(RPU_VERSION).tgz -C build rpu-$(RPU_VERSION)

clean: # Clean up build and release artifacts
	rm -rf build target
