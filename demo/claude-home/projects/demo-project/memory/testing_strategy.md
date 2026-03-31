---
name: Testing Strategy
description: Integration tests must use real database, learned from past incident
type: feedback
---

Integration tests must hit a real database, not mocks.

**Why:** Got burned last quarter when mocked tests passed but a production migration failed due to Prisma schema divergence. The mock didn't catch a breaking change in a foreign key constraint.

**How to apply:** When writing database tests, always use the test database with real Prisma migrations. Only mock external HTTP APIs (use msw), never the data layer.
