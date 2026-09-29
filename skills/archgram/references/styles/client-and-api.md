# Client and API

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
