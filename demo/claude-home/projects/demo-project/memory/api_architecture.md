---
name: API Architecture
description: REST API migration from Express to Fastify, decision context
type: project
---

Migrating REST API from Express to Fastify for better performance and schema validation.

**Why:** Express middleware chain is becoming a bottleneck. Fastify's schema-based validation and serialization give 2-3x throughput improvement. Migration approved by tech lead on 2026-03-15.

**How to apply:** New endpoints should use Fastify patterns (schema declaration, reply.send). Don't refactor existing Express routes unless touching them for other reasons.
