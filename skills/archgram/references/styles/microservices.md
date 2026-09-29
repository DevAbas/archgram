# Microservices

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
