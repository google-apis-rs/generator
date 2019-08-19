MCPD = target/debug/mcp
MCP = target/release/mcp
.PHONY = always # TODO: one day we should be precise, and provide actual dependencies so 'make' can be smart
API_INDEX_JSON = etc/api-index.v1.json
API_INDEX_MAPPED_JSON = etc/api-index-mapped.v1.json
GEN_DIR = gen
MAKEFILE_TPL = templates/Makefile.liquid
GEN_MAKEFILE = $(GEN_DIR)/Makefile

help:
	$(info -- Targets for development -----------------------------------------------------------)
	$(info tests                      | run *all* the tests)
	$(info mcp-tests                  | run all tests for the 'master control program')
	$(info cargo-tests                | run all tests driven by cargo)
	$(info -- Targets for files we depend on ----------------------------------------------------)
	$(info update-all-metadata        | invalidate all specifications from google and fetch the latest versions)
	$(info fetch-api-specs            | fetch all apis our local discovery document knows, and store them in $(GEN_DIR))
	$(info generate-gen-makefile      | a makefile containing useful targets to build and test generated crates)
	$(info -- Everything Else -------------------------------------------------------------------)
	$(info pull-generated             | be sure the 'generated' repository is latest)
	$(info update-generated-fixtures  | update everything that was generated in <this> repository)
	$(info --------------------------------------------------------------------------------------)

always:

$(MCPD): always
	cargo build

$(MCP): always
	cargo build --release

$(API_INDEX_JSON):
	curl -S https://www.googleapis.com/discovery/v1/apis > $@

$(API_INDEX_MAPPED_JSON): $(API_INDEX_JSON) $(MCPD) 
	$(MCPD) map-api-index $< $@

$(GEN_DIR):
	git clone --depth=1 https://github.com/google-apis-rs/generated $@

pull-generated: $(GEN_DIR)
	cd $(GEN_DIR) && git pull --ff-only

update-all-metadata:
	rm $(API_INDEX_JSON)
	$(MAKE) fetch-api-specs

update-generated-fixtures: tests/mcp/fixtures/shared/known-versions discovery_parser/src/discovery.rs

tests/mcp/fixtures/shared/known-versions: $(API_INDEX_JSON)
	# version 1.6 known to be working
	jq -r '.items[].version' < $(API_INDEX_JSON) | sort | uniq > $@

discovery_parser/src/discovery.rs: $(API_INDEX_JSON)
	# version 15.0.199 known to be working
	quicktype --lang rust --visibility=public $(API_INDEX_JSON) > $@

api-index: $(API_INDEX_JSON) $(GEN_MAKEFILE)

fetch-api-specs: api-index $(MCP) $(GEN_DIR)
	$(MCP) fetch-api-specs $(API_INDEX_JSON) $(GEN_DIR)

mcp-tests: $(MCPD)
	tests/mcp/journey-tests.sh $<

cargo-tests:
	cargo test --lib --bin mcp --all --examples

tests: mcp-tests cargo-tests

$(GEN_MAKEFILE): $(API_INDEX_MAPPED_JSON) $(MCPD) $(GEN_DIR) $(MAKEFILE_TPL) 
	$(MCPD) substitute $(MAKEFILE_TPL):$@ < $< 
	
generate-gen-makefile: $(GEN_MAKEFILE)
