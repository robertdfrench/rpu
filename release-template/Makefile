PREFIX=/usr/local
MANPATH=$(PREFIX)/share/man/man6
BINPATH=$(PREFIX)/bin

install: $(MANPATH)/rpu.6 $(BINPATH)/rpu
        @echo "Run 'man rpu' for usage information"

$(MANPATH)/rpu.6: rpu.6
        install -o root -g root -m 0755 -d $(MANPATH)
        install -o root -g root -m 0644 rpu.6 $@

$(BINPATH)/rpu: rpu
        install -o root -g root -m 0755 rpu $@
