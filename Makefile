MCPD = target/debug/mcp
.PHONY = always

help:
	$(info -- Targets for development -----------------------------------------------------------)
	$(info mcp-tests                  | run all tests for the 'master control program'           )

always:

$(MCPD): always
	cargo build --bin mcp

mcp-tests: $(MCPD)
	tests/mcp/journey-tests.sh $<
	

