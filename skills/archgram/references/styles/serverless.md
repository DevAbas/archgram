# Serverless

- **Recognise:** functions as the unit of deployment (`functions/`,
  `handler.ts`, `serverless.yml`, SAM, SST, Cloudflare Workers) wired to
  triggers: HTTP, a queue, a bucket, a schedule.
- **Parts:** triggers, functions, managed stores and services.
- **Questions:** what starts each function, and what does it touch?
- **Draw:** triggers as the nodes that lead in; functions with the same
  trigger and the same stores are one node; managed services as external.
