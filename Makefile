help: #: Display this help menu
	@echo "USAGE:\n"
	@cat $(MAKEFILE_LIST) \
		| grep -v 'awk' \
		| awk -F':' '/#:/ { OFS=":"; print " ",$$1,$$3 }' \
		| column -s':' -t \
		| sort

PREFIX=/usr/local
MANPATH=$(PREFIX)/share/man/man6
BINPATH=$(PREFIX)/bin

install: $(MANPATH)/rpu.6 $(BINPATH)/rpu  #: Install rpu
	@echo "Run 'man rpu' for usage information"

$(MANPATH)/rpu.6: rpu.6
	install -o root -g root -m 0755 -d $(MANPATH)
	install -o root -g root -m 0644 rpu.6 $@

$(BINPATH)/rpu: rpu
	install -o root -g root -m 0755 rpu $@


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
	cp Makefile build/rpu-$(RPU_VERSION)
	cp README.md build/rpu-$(RPU_VERSION)
	cp rpu.6 build/rpu-$(RPU_VERSION)
	cp examples/* build/rpu-$(RPU_VERSION)/examples
	cp target/release/rpu build/rpu-$(RPU_VERSION)
	tar -czf build/rpu-$(RPU_VERSION).tgz -C build rpu-$(RPU_VERSION)

clean: # Clean up build and release artifacts
	rm -rf build target
