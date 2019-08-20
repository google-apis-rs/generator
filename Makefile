MCPD = target/debug/mcp
MCP = target/release/mcp
.PHONY = always # TODO: one day we should be precise, and provide actual dependencies so 'make' can be smart

help:
	$(info -- Targets for development -----------------------------------------------------------)
	$(info tests                      | run *all* the tests)
	$(info mcp-tests                  | run all tests for the 'master control program')
	$(info cargo-tests                | run all tests driven by cargo)
	$(info -- Everything Else -------------------------------------------------------------------)
	$(info update-generated-fixtures  | update everything that was generated in <this> repository)
	$(info --------------------------------------------------------------------------------------)

always:

$(MCPD): always
	cargo build

$(MCP): always
	cargo build --release

update-generated-fixtures: shared/tests/fixtures/known-versions discovery_parser/src/discovery.rs

shared/tests/fixtures/known-versions: $(API_INDEX_JSON)
	# version 1.6 known to be working
	jq -r '.items[].version' < $(API_INDEX_JSON) | sort | uniq > $@

discovery_parser/src/discovery.rs: $(API_INDEX_JSON)
	# version 15.0.199 known to be working
	quicktype --lang rust --visibility=public $(API_INDEX_JSON) > $@

mcp-tests: $(MCPD)
	tests/mcp/journey-tests.sh $<

cargo-tests:
	cargo test --lib --bin mcp --all --examples

tests: mcp-tests cargo-tests
