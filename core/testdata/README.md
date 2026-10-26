The 32 cases in `rust-subscriptions.jsonl` were captured from commit
`0c8f0ce800d5959f41ff35d67622af3d108992e9` by wrapping `parse_remote`
and `build_config` while running all subscription tests in the original Rust core.
Successful outputs are compared structurally, preserving array order; rejected
inputs must remain rejected. Timestamps are normalized by the Go test.

One intentional correction: IPv6 link server addresses exclude URL brackets.
The original Rust suite itself failed its IPv6 assertion (`[2001:db8::1]` versus
`2001:db8::1`); the fixture follows the intended address in that assertion.
