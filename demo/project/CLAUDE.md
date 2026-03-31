# Acme Web Platform

A full-stack web application built with React + Node.js + PostgreSQL.

## Build & Test
```sh
npm run dev          # start dev server
npm test             # run test suite
npm run lint         # eslint + prettier
```

## Architecture
- `src/api/` — Express REST API with JWT auth
- `src/web/` — React SPA with TypeScript
- `src/db/` — Prisma ORM, migrations in `prisma/migrations/`

## Conventions
- Use functional React components with hooks
- API routes follow REST conventions: `GET /api/v1/resource`
- All database queries go through Prisma, never raw SQL
- Tests use vitest for unit tests, playwright for e2e
