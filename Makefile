PACKAGES=by-macros dioxus-gtag-macro dioxus-gtag

setup:
	cargo install mdbook

.PHONY: publish
publish: $(patsubst %,publish.%,$(PACKAGES))

publish.%:
	./publish.sh $*

.PHONY: build
build: $(patsubst %,build.%,$(PACKAGES))

build.%:
	cargo build -p $*

.PHONY: docs
docs:
	mdbook serve
