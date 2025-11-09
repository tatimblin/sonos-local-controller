# Makefile for Sonos library testing

.PHONY: test test-unit test-integ test-all

# Run only unit tests
test-unit:
	cargo test --lib

# Run only integration tests  
test-integ:
	cargo test --tests

# Run all tests (unit + integration)
test-all:
	cargo test

# Default test command (unit tests only)
test: test-unit

# Individual integration test files
test-discovery:
	cargo test --test discovery

test-soap:
	cargo test --test soap