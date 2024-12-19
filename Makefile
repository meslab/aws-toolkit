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
	
ssm-session:
	cargo update
	cargo build -r --bin ssm-session
	strip target/release/ssm-session

scale-in-ecs:
	cargo update
	cargo build -r --bin scale-in-ecs
	strip target/release/scale-in-ecs

ecr-gitconfig:
	cargo update
	cargo build -r --bin ecr-gitconfig
	strip target/release/ecr-gitconfig

ses-suppression-list:
	cargo update
	cargo build -r --bin ses-suppression-list
	strip target/release/ses-suppression-list

release: ssm-session scale-in-ecs ecr-gitconfig ses-suppression-list

all:
	cargo build -r
	strip target/release/ssm-session
	strip target/release/scale-in-ecs
	strip target/release/ecr-gitconfig
	strip target/release/ses-suppression-list

install: all
	cp target/release/ssm-session ~/.local/bin
	cp target/release/scale-in-ecs ~/.local/bin
	cp target/release/ecr-gitconfig ~/.local/bin
	cp target/release/ses-suppression-list ~/.local/bin

clean:
	cargo clean
	rm -rf target

uninstall: clean
	rm -f ~/.local/bin/ssm-session
	rm -f ~/.local/bin/scale-in-ecs
	rm -f ~/.local/bin/ecr-gitconfig
	rm -f ~/.local/bin/ses-suppression-list

.PHONY: rust-version format lint test run build ssm-session scale-in-ecs ecr-gitconfig ses-suppression-list release install clean uninstall all
