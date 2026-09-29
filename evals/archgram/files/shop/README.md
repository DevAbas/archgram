# shop

A small store. The API serves the catalogue from Postgres, with a Redis cache in front of it, and puts each order on a queue; a worker takes orders off the queue and writes them to Postgres.
