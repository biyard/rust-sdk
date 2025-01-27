PACKAGES=by-types by-macros by-axum rest-api dioxus-oauth dioxus-popup dioxus-translate-types dioxus-translate-macro dioxus-translate google-wallet

.PHONY: publish
publish: $(patsubst %,publish.%,$(PACKAGES))

publish.%:
	./publish.sh $*

.PHONY: build
build: $(patsubst %,build.%,$(PACKAGES))

build.%:
	cargo build -p $*
