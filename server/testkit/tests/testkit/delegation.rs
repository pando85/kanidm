// TODO: Integration tests for delegated administration need to be implemented.
//
// These tests should:
// 1. Create delegated access controls with scope groups and/or scope filters
// 2. Create a delegated admin user who is a member of the delegation scope group
// 3. Authenticate as the delegated admin (not ADMIN_TEST_USER)
// 4. Test that the delegated admin can perform operations within their scope
// 5. Test that the delegated admin cannot perform operations outside their scope
//
// The unit tests in server/lib/src/server/access/mod.rs provide good coverage
// for the access control logic. Integration tests should focus on the end-to-end
// workflow of delegated administration.
//
// See existing tests in person.rs and group.rs for patterns on how to:
// - Create access controls via the API
// - Switch authentication to a different user
// - Verify permissions are correctly enforced
