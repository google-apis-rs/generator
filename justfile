api_index_path := "../generated/etc/api-index.v1.json"
mcpd := "target/debug/mcp"

# run *all* the tests
tests: mcp-tests cargo-tests

# run all tests for the 'master control program'
mcp-tests:  mcpd
	tests/mcp/journey-tests.sh {{mcpd}}

# run all tests driven by cargo
cargo-tests:
	cargo test --tests --examples --all-features

# update everything that was generated in <this> repository
update-generated-fixtures: discovery-spec known-versions-fixture discovery-rs

# build the master control program in debug mode
mcpd:
    cargo build

# fetch the spec used as fixture in our tests
discovery-spec:
	curl https://www.googleapis.com/discovery/v1/apis/admin/directory_v1/rest -o discovery_parser/tests/spec.json

# Update a fixture with all API versions encountered in the Google API index
known-versions-fixture:
	# version 1.6 known to be working
	jq -r '.items[].version' < {{api_index_path}} | sort | uniq > shared/tests/fixtures/known-versions

# A generated file with types supported the deserializtion of the Google API index
discovery-rs:
	# version 15.0.199 known to be working
	quicktype --lang rust --visibility=public {{api_index_path}} > discovery_parser/src/discovery.rs

