MCPD = target/debug/mcp
.PHONY = always
DISCOVERY_APIS_JSON = etc/discovery-apis-v1.json

help:
	$(info -- Targets for development -----------------------------------------------------------)
	$(info run all the tests          | run all tests for the 'master control program')
	$(info mcp-tests                  | run all tests for the 'master control program')
	$(info cargo-tests                | run all tests driven by cargo)
	$(info -- Targets for files we depend on ----------------------------------------------------)
	$(info discovery-apis             | Fetch the list of available Google APIs)
	$(info --------------------------------------------------------------------------------------)

always:

$(MCPD): always
	cargo build

$(DISCOVERY_APIS_JSON):
	curl -S https://www.googleapis.com/discovery/v1/apis > $@

discovery_parser/src/discovery.rs: $(DISCOVERY_APIS_JSON)
	# quicktype version 15.0.199 known to be working
	quicktype --lang rust --visibility=public $(DISCOVERY_APIS_JSON) > $@

discovery-apis: $(DISCOVERY_APIS_JSON)

mcp-tests: $(MCPD)
	tests/mcp/journey-tests.sh $<

cargo-tests:
	cargo test --lib --all

tests: mcp-tests cargo-tests
	

