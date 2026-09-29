# Layered

- **Recognise:** folders by technical role (`controllers/`, `services/`,
  `repositories/`, `models/`); each layer imports only the one below.
- **Parts:** the entry layer (routes, controllers), the business layer,
  data access, the database.
- **Questions:** where does a request go down, and where does the
  business rule live? Does any layer skip the one below?
- **Draw:** one node per layer at container level, not one per class;
  `direction: down`. A layer skipped is the one idea when it happens.
