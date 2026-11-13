// Ported C++ gtests (test/actor and the thread/loop parts of test/util);
// markers `// gtest: Group.Name`. One test at a time (RUST_TEST_THREADS in
// .cargo/config.toml), as gtest runs them: they share the background pool and
// the sequenced schedulers.
mod gtest;
// Port-only: a cooperative scheduler installed for the whole process
// (tests/cooperative.rs), so apart from the tests that start threads.
mod cooperative;
