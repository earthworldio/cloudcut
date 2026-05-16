# cloudcut-challenge

A multi-package Cargo Workspace monorepo for a SaaS project.

## Project Structure

- `backend/api`: Binary crate for the API service.
- `backend/worker`: Binary crate for the background worker.
- `backend/shared`: Library crate for shared logic and data structures.
- `frontend`: Vite + React + TypeScript frontend.
- `docker-compose.yml`: Infrastructure setup (PostgreSQL, Redis).
- `DESIGN.md`: Architecture and design documentation.
