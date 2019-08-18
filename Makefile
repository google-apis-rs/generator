MCPD = target/debug/mcp
MCP = target/release/mcp
.PHONY = always # TODO: one day we should be precise, and provide actual dependencies so 'make' can be smart
DISCOVERY_APIS_JSON = etc/discovery-apis-v1.json
API_DIR = etc/api

help:
	$(info -- Targets for development -----------------------------------------------------------)
	$(info tests                      | run *all* the tests)
	$(info mcp-tests                  | run all tests for the 'master control program')
	$(info cargo-tests                | run all tests driven by cargo)
	$(info -- Targets for files we depend on ----------------------------------------------------)
	$(info fetch-api-specs            | fetch all apis our discovery document knows, and store them in $(API_DIR))
	$(info discovery-apis             | fetch the list of available Google APIs)
	$(info --------------------------------------------------------------------------------------)

always:

$(MCPD): always
	cargo build

$(MCP): always
	cargo build --release

$(DISCOVERY_APIS_JSON):
	curl -S https://www.googleapis.com/discovery/v1/apis > $@

discovery_parser/src/discovery.rs: $(DISCOVERY_APIS_JSON)
	# quicktype version 15.0.199 known to be working
	quicktype --lang rust --visibility=public $(DISCOVERY_APIS_JSON) > $@

discovery-apis: $(DISCOVERY_APIS_JSON)

fetch-api-specs: $(DISCOVERY_APIS_JSON) $(MCP)
	$(MCP) fetch-specs $(DISCOVERY_APIS_JSON) $(API_DIR)

mcp-tests: $(MCPD)
	tests/mcp/journey-tests.sh $<

cargo-tests:
	cargo test --lib --all

tests: mcp-tests cargo-tests
	

