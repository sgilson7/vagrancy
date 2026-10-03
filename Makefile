ROOT := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
PY := $(ROOT)/.venv-test/bin/python

.PHONY: help test check web serve test-ui test-ui-online referee test-ui-setup ladder count publish clean

## test: the whole suite, native, no window and no network
test:
	@cargo test --workspace

## check: a fast type-check, no binaries produced
check:
	@cargo check --workspace --all-targets

## web: the browser build into dist/web/
web:
	@$(ROOT)/packaging/package-web.sh

## serve: build and serve the app locally on http://localhost:8080/
serve: web
	@echo "Serving http://localhost:8080/ - Ctrl-C to stop"
	@cd $(ROOT)/dist/web && python3 -m http.server 8080

## test-ui: walk the gate in Chromium, Firefox and WebKit
test-ui: web
	@$(PY) $(ROOT)/testing/drive.py chromium firefox webkit

## test-ui-online: two tabs play one match online, by pasted codes, in three engines
test-ui-online: web
	@$(PY) $(ROOT)/testing/online.py chromium firefox webkit

## referee: a scripted three-minute online match, written to analysis/referee.md
referee: web
	@$(PY) $(ROOT)/testing/online.py chromium --seconds 180

## test-ui-setup: one-time install of Playwright and its three engines
test-ui-setup:
	@python3 -m venv $(ROOT)/.venv-test
	@$(ROOT)/.venv-test/bin/pip -q install playwright
	@$(ROOT)/.venv-test/bin/playwright install chromium firefox webkit
	@echo "ready: make test-ui"

## ladder: play the yardstick pilot against every opponent; writes analysis/ladder.md
ladder:
	@cargo run -q --release -p lab -- ladder

## count: how many tests there are (packaging/count-tests.sh)
count:
	@$(ROOT)/packaging/count-tests.sh

## publish: run the suite and push; Actions builds and deploys (Sam's)
# Sam's command. The agent pushes only on Sam's explicit ask (CLAUDE.md).
publish:
	@cargo test --workspace --quiet
	@git push
	@echo "Pushed. Actions tests, builds, walks the gate and deploys."

## clean: remove build output
clean:
	@rm -rf $(ROOT)/dist $(ROOT)/target

help:
	@grep -hE '^## ' $(MAKEFILE_LIST) | sed 's/## //' | awk -F': ' '{printf "  \033[1m%-14s\033[0m %s\n", $$1, $$2}'
