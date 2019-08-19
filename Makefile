MCPD = target/debug/mcp
MCP = target/release/mcp
.PHONY = always # TODO: one day we should be precise, and provide actual dependencies so 'make' can be smart
API_INDEX_JSON = etc/api-index.v1.json
API_DIR = etc/api
GEN_DIR = gen

help:
	$(info -- Targets for development -----------------------------------------------------------)
	$(info tests                      | run *all* the tests)
	$(info mcp-tests                  | run all tests for the 'master control program')
	$(info cargo-tests                | run all tests driven by cargo)
	$(info -- Targets for files we depend on ----------------------------------------------------)
	$(info update-all-metadata        | invalidate all specifications from google and fetch the latest versions
	$(info fetch-api-specs            | fetch all apis our local discovery document knows, and store them in $(API_DIR))
	$(info -- Everything Else -------------------------------------------------------------------)
	$(info pull-generated             | be sure the generated repository is latest)
	$(info --------------------------------------------------------------------------------------)

always:

$(MCPD): always
	cargo build

$(MCP): always
	cargo build --release

$(API_INDEX_JSON):
	curl -S https://www.googleapis.com/discovery/v1/apis > $@

$(GEN_DIR):
	git clone --depth=1 https://github.com/google-apis-rs/generated $@

pull-generated: $(GEN_DIR)
	cd $(GEN_DIR) && git pull --ff-only

update-all-metadata:
	rm $(API_INDEX_JSON)
	$(MAKE) fetch-api-specs

discovery_parser/src/discovery.rs: $(API_INDEX_JSON)
	# quicktype version 15.0.199 known to be working
	quicktype --lang rust --visibility=public $(API_INDEX_JSON) > $@

api-index: $(API_INDEX_JSON)

fetch-api-specs: $(API_INDEX_JSON) $(MCP)
	$(MCP) fetch-api-specs $(API_INDEX_JSON) $(API_DIR)

mcp-tests: $(MCPD)
	tests/mcp/journey-tests.sh $<

cargo-tests:
	cargo test --lib --bin mcp --all

tests: mcp-tests cargo-tests
	

