rust-version:
	rustc --version 		# rustc compiler
	cargo --version 		# cargo package manager
	rustfmt --version 		# rust formatter
	rustup --version 		# rust toolchain manager
	clippy-driver --version	# rust linter

format:
	cargo fmt

lint:
	cargo clippy

test:
	cargo test

run:
	cargo run -r

build:
	cargo update
	cargo build 
	
release:
	cargo update
	cargo build -r
	strip target/release/ssm-session
	strip target/release/scale-in-ecs

install: release
	cp target/release/ssm-session ~/.local/bin
	cp target/release/scale-in-ecs ~/.local/bin

clean:
	cargo clean
	rm -rf target

uninstall: clean
	rm -f ~/.local/bin/ssm-session
	rm -f ~/.local/bin/scale-in-ecs
