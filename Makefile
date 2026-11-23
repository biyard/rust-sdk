PACKAGES=by-macros dioxus-gtag-macro dioxus-gtag ts-server-fn-client ts-server-fn ts-server-fn-axum

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

.PHONY: test.grpc
test.grpc:
	cargo fmt -p by-grpc -p by-grpc-macros --check
	cargo clippy -p by-grpc -p by-grpc-macros --all-targets -- -D warnings
	cargo test -p by-grpc -p by-grpc-macros
	cd examples/example-grpc-server && cargo fmt --check
	cd examples/example-grpc-server && cargo clippy --locked --all-targets -- -D warnings
	cd examples/example-grpc-server && cargo test --locked

.PHONY: test.e2e
test.e2e:
	cd fixtures/ts-server-fn-e2e/service && cargo test --locked && cargo check --no-default-features --locked
	cd fixtures/ts-server-fn-e2e/client && cargo test --locked
