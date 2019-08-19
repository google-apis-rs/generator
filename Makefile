MCPD = target/debug/mcp
MCP = target/release/mcp
.PHONY = always # TODO: one day we should be precise, and provide actual dependencies so 'make' can be smart
API_INDEX_JSON = etc/api-index.v1.json
API_DIR = etc/api

help:
	$(info -- Targets for development -----------------------------------------------------------)
	$(info tests                      | run *all* the tests)
	$(info mcp-tests                  | run all tests for the 'master control program')
	$(info cargo-tests                | run all tests driven by cargo)
	$(info -- Targets for files we depend on ----------------------------------------------------)
	$(info fetch-api-specs            | fetch all apis our discovery document knows, and store them in $(API_DIR))
	$(info api-index                  | fetch the list of available Google APIs)
	$(info --------------------------------------------------------------------------------------)

always:

$(MCPD): always
	cargo build

$(MCP): always
	cargo build --release

$(API_INDEX_JSON):
	curl -S https://www.googleapis.com/discovery/v1/apis > $@

discovery_parser/src/discovery.rs: $(API_INDEX_JSON)
	# quicktype version 15.0.199 known to be working
	quicktype --lang rust --visibility=public $(API_INDEX_JSON) > $@

api-index: $(API_INDEX_JSON)

fetch-api-specs: $(API_INDEX_JSON) $(MCP)
	$(MCP) fetch-api-specs $(API_INDEX_JSON) $(API_DIR)

mcp-tests: $(MCPD)
	tests/mcp/journey-tests.sh $<

cargo-tests:
	cargo test --lib --all

tests: mcp-tests cargo-tests
	

