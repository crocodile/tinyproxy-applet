CARGO ?= cargo
APPLET_BIN := tinyproxy-applet
SYSTEM_FALLBACK_BINDIR ?= /usr/local/bin

INSTALLED_SYSTEM_BINDIR := $(shell if [ -x /usr/bin/$(APPLET_BIN) ]; then printf '%s' /usr/bin; elif [ -x /usr/local/bin/$(APPLET_BIN) ]; then printf '%s' /usr/local/bin; else printf '%s' $(SYSTEM_FALLBACK_BINDIR); fi)

.PHONY: build install uninstall clean

build:
	$(CARGO) build --release

install: build
	install -Dm755 target/release/$(APPLET_BIN) $(DESTDIR)$(INSTALLED_SYSTEM_BINDIR)/$(APPLET_BIN)

uninstall:
	rm -f $(DESTDIR)$(INSTALLED_SYSTEM_BINDIR)/$(APPLET_BIN)

clean:
	$(CARGO) clean
