# Styles of architecture

Most systems follow one style, or a few side by side. Recognising it tells
you the parts to look for, the questions a reader brings, and what can be
merged. Read the entry for the style you recognise; a system that fits
none is drawn from `references/architecture.md` alone.

The styles follow the usual catalogues: Richards and Ford's
*Fundamentals of Software Architecture*, Buschmann et al.'s
*Pattern-Oriented Software Architecture* (pipes and filters, microkernel),
Cockburn's hexagonal architecture, and the C4 model for the levels.

Each entry says how to **recognise** it in the code, its usual **parts**,
the **questions** its reader asks, and how to **draw** it.

## Contents

- Layered
- Hexagonal (ports and adapters)
- Client and API
- Microservices
- Event-driven
- Pipeline (pipes and filters)
- Serverless
- Plugin host (microkernel)
- Compiler, CLI and build tool
- Libraries in a monorepo
- LLM, retrieval and agents
- Several styles at once

## Layered

- **Recognise:** folders by technical role (`controllers/`, `services/`,
  `repositories/`, `models/`); each layer imports only the one below.
- **Parts:** the entry layer (routes, controllers), the business layer,
  data access, the database.
- **Questions:** where does a request go down, and where does the
  business rule live? Does any layer skip the one below?
- **Draw:** one node per layer at container level, not one per class;
  `direction: down`. A layer skipped is the one idea when it happens.

## Hexagonal (ports and adapters)

- **Recognise:** a core with no framework imports (`domain/`, `core/`),
  interfaces it owns (`ports/`), and implementations outside it
  (`adapters/`, `infrastructure/`); also "clean" or "onion" architecture.
- **Parts:** driving adapters (HTTP, CLI, a queue consumer), the core,
  driven adapters (database, email, a payment API).
- **Questions:** what may the core depend on? What can be swapped?
- **Draw:** the core in the middle, driving adapters before it, driven
  adapters after it; adapters of one kind with the same relations are one
  node. The one idea is usually that every arrow points into the core's
  ports.

## Client and API

- **Recognise:** a front end (`app/`, `pages/`, `src/components/`, a
  mobile project) and a server it calls (`api/`, route handlers, a
  separate service); `fetch` or a generated client between them.
- **Parts:** the client, the API, its stores, the external services it
  calls, the identity provider.
- **Questions:** what runs in the browser, what on the server, what in a
  third party? Where is the state, and where is the trust boundary?
- **Draw:** a frame per place that runs code (browser, server, third
  party); one flow for the main user action. Pages are not nodes; the
  routes that matter are, or the edge labels name them.

## Microservices

- **Recognise:** several deployable services, each with its own
  manifest, `Dockerfile` or chart; a `docker-compose.yml`, Kubernetes
  manifests or a service mesh; each service with its own store.
- **Parts:** a gateway or front door, the services, each service's
  store, the broker between them, external systems.
- **Questions:** which service owns which data? What is a call and what a
  message? What happens when one is down?
- **Draw:** one node per service and one per store it owns, at container
  level; services that only differ in what they serve and share their
  relations may be one multi-node. Synchronous calls and messages are
  told apart by the queue node between them, not by edge style.

## Event-driven

- **Recognise:** producers and consumers of named events or topics; a
  broker (Kafka, RabbitMQ, SQS, NATS, Redis streams); handlers registered
  by event name; sometimes an event store.
- **Parts:** producers, the broker or topics, consumers, what consumers
  write.
- **Questions:** who reacts to what? What happens if a consumer fails,
  and in what order do things happen?
- **Draw:** the broker or each topic that matters as a queue node between
  producers and consumers; a flow per event the reader asks about.
  Consumers of one event with the same relations are one node.

## Pipeline (pipes and filters)

- **Recognise:** stages that each take data in and pass it on: ETL jobs,
  `extract`/`transform`/`load`, DAGs (Airflow, Dagster, dbt), stream
  processors, a scheduler.
- **Parts:** sources, stages, the stores between them, the destination,
  where bad data goes, what schedules it.
- **Questions:** where does data enter, change and land? Batch or
  stream, on what schedule? Where is it checked?
- **Draw:** `direction: right`, one node per stage the reader must tell
  apart; a run of stages with nothing between them is one node. The flow
  is one record's way through.

## Serverless

- **Recognise:** functions as the unit of deployment (`functions/`,
  `handler.ts`, `serverless.yml`, SAM, SST, Cloudflare Workers) wired to
  triggers: HTTP, a queue, a bucket, a schedule.
- **Parts:** triggers, functions, managed stores and services.
- **Questions:** what starts each function, and what does it touch?
- **Draw:** triggers as the nodes that lead in; functions with the same
  trigger and the same stores are one node; managed services as external.

## Plugin host (microkernel)

- **Recognise:** a host that loads extensions by manifest or registry
  (`plugins/`, `extensions/`, a `plugin.json`, entry points declared in a
  package), with an API the extensions call back.
- **Parts:** the host, the extension API, the extensions, what each adds,
  and the files an extension installs into the host's project.
- **Questions:** what does the host own, and what does an extension add?
  What does installing one change?
- **Draw:** the host and its API as the core; extensions with the same
  relations as one node; what an extension writes into a project in a
  frame for that project. Where the extension's files are installed
  elsewhere, name the installed path and say where the source lives.

## Compiler, CLI and build tool

- **Recognise:** a command that reads input, runs it through stages and
  writes output: parse, validate, transform, render or emit; a `bin`
  entry; subcommands.
- **Parts:** the input, each stage, the output, and the tools or files
  each stage reads.
- **Questions:** what does it read and write? In which stage is each
  decision made? What is generated and must not be edited?
- **Draw:** `direction: right`, one node per stage; subcommands that run
  the same stages are one entry node, their names in its note.

## Libraries in a monorepo

- **Recognise:** workspaces (`packages/`, `apps/`, `crates/`, a
  `pnpm-workspace.yaml`, a Cargo workspace), each with its own manifest.
- **Parts:** the apps, the shared packages, and which depends on which.
- **Questions:** what may depend on what? What does each app pull in?
- **Draw:** one node per package, edges for dependencies only, with that
  meaning said in the description. Leaf utilities with the same
  dependents are one node.

## LLM, retrieval and agents

- **Recognise:** calls to a model API, embeddings, a vector store,
  prompt files, tool definitions, an agent loop.
- **Parts:** what is prepared ahead (ingest, chunk, embed, index), what
  happens at question time (retrieve, prompt, call, check), the tools the
  model may call, what the answer is checked against.
- **Questions:** what does the model see, and never see? Which steps are
  model calls, and which plain code? Exact or semantic retrieval? What
  stops it inventing facts?
- **Draw:** a frame for ahead of time and one for question time, the
  stores between them; every model call its own node, so a reader can
  count them. Tools with the same relations are one node.

## Several styles at once

A real system mixes them: a client and API in front of an event-driven
back end, a pipeline that feeds a retrieval system. Draw each at the
level the reader needs, and when two styles each need their own flow and
their own frames, prefer two diagrams to one crowded one.
