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

s3-guardduty-copy:
	cargo update
	cargo build -r --bin s3-guardduty-copy
	strip target/release/s3-guardduty-copy

release: ssm-session scale-in-ecs ecr-gitconfig ses-suppression-list s3-guardduty-copy

all:
	cargo build -r
	strip target/release/ssm-session
	strip target/release/scale-in-ecs
	strip target/release/ecr-gitconfig
	strip target/release/ses-suppression-list
	strip target/release/s3-guardduty-copy

install: all
	cp target/release/ssm-session ~/.local/bin
	cp target/release/scale-in-ecs ~/.local/bin
	cp target/release/ecr-gitconfig ~/.local/bin
	cp target/release/ses-suppression-list ~/.local/bin
	cp target/release/release-codepipelines ~/.local/bin
	cp target/release/s3-guardduty-copy ~/.local/bin

clean:
	cargo clean
	rm -rf target

uninstall: clean
	rm -f ~/.local/bin/ssm-session
	rm -f ~/.local/bin/scale-in-ecs
	rm -f ~/.local/bin/ecr-gitconfig
	rm -f ~/.local/bin/ses-suppression-list
	rm -f ~/.local/bin/release-codepipelines
	rm -f ~/.local/bin/s3-guardduty-copy

upgrade:
	rustup update
	for i in $$(cat Cargo.toml | grep '^aws-' | awk '{ print $$1 }'); do cargo remove $$i; cargo add $$i -F behavior-version-latest; done
	cargo update

.PHONY: rust-version format lint test run build ssm-session scale-in-ecs ecr-gitconfig ses-suppression-list s3-guardduty-copy release install clean uninstall all upgrade
