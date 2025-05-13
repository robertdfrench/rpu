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


test: #: Any tests runnable by cargo
	cargo test

release: test #: Build a release tarball for installation
ifndef VERSION
	$(error "Bud, c'mon pal you gotta define VERSION first")
endif
	cargo build --release
	mkdir -p build
	mkdir -p build/rpu-$(VERSION)
	mkdir -p build/rpu-$(VERSION)/examples
	cp Makefile build/rpu-$(VERSION)
	cp README.md build/rpu-$(VERSION)
	cp rpu.6 build/rpu-$(VERSION)
	cp examples/* build/rpu-$(VERSION)/examples
	cp target/release/rpu build/rpu-$(VERSION)
	tar -czf build/rpu-$(VERSION).tgz -C build rpu-$(VERSION)

clean: # Clean up build and release artifacts
	rm -rf build target
