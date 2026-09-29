# Styles of architecture

Most systems follow one style, or a few side by side. Recognising it tells
you the parts to look for, the questions a reader brings, and what can be
merged. Find the style in the table by what the code shows, then read its
file, and only that one; a system that fits none is drawn from
`references/architecture.md` alone. Say in the report which style you
recognised, or that none fitted.

The styles follow the usual catalogues: Richards and Ford's
*Fundamentals of Software Architecture*, Buschmann et al.'s
*Pattern-Oriented Software Architecture* (pipes and filters, microkernel),
Cockburn's hexagonal architecture, and the C4 model for the levels. Each
file says how to recognise the style, its usual parts, its reader's
questions, and how to draw it.

| Style | You see in the code | Read |
|---|---|---|
| Layered | folders by technical role (`controllers/`, `services/`, `repositories/`, `models/`); each layer imports only the one below | `references/styles/layered.md` |
| Hexagonal (ports and adapters) | a core with no framework imports (`domain/`, `core/`), interfaces it owns (`ports/`), and implementations outside it (`adapters/`, `infrastructure/`); also "clean" or "onion" architecture | `references/styles/ports-and-adapters.md` |
| Client and API | a front end (`app/`, `pages/`, `src/components/`, a mobile project) and a server it calls (`api/`, route handlers, a separate service); `fetch` or a generated client between them | `references/styles/client-and-api.md` |
| Microservices | several deployable services, each with its own manifest, `Dockerfile` or chart; a `docker-compose.yml`, Kubernetes manifests or a service mesh; each service with its own store | `references/styles/microservices.md` |
| Event-driven | producers and consumers of named events or topics; a broker (Kafka, RabbitMQ, SQS, NATS, Redis streams); handlers registered by event name; sometimes an event store | `references/styles/event-driven.md` |
| Pipeline (pipes and filters) | stages that each take data in and pass it on: ETL jobs, `extract`/`transform`/`load`, DAGs (Airflow, Dagster, dbt), stream processors, a scheduler | `references/styles/pipeline.md` |
| Serverless | functions as the unit of deployment (`functions/`, `handler.ts`, `serverless.yml`, SAM, SST, Cloudflare Workers) wired to triggers: HTTP, a queue, a bucket, a schedule | `references/styles/serverless.md` |
| Plugin host (microkernel) | a host that loads extensions by manifest or registry (`plugins/`, `extensions/`, a `plugin.json`, entry points declared in a package), with an API the extensions call back | `references/styles/plugin-host.md` |
| Compiler, CLI and build tool | a command that reads input, runs it through stages and writes output: parse, validate, transform, render or emit; a `bin` entry; subcommands | `references/styles/compiler-and-cli.md` |
| Libraries in a monorepo | workspaces (`packages/`, `apps/`, `crates/`, a `pnpm-workspace.yaml`, a Cargo workspace), each with its own manifest | `references/styles/monorepo.md` |
| LLM, retrieval and agents | calls to a model API, embeddings, a vector store, prompt files, tool definitions, an agent loop | `references/styles/llm-and-retrieval.md` |

## Several styles at once

A real system mixes them: a client and API in front of an event-driven
back end, a pipeline that feeds a retrieval system. Draw each at the
level the reader needs, and when two styles each need their own flow and
their own frames, prefer two diagrams to one crowded one.
