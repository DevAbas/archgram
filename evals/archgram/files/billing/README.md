# Billing

Bills a customer's usage: it computes the invoice, charges the card and
emails the receipt. The billing rules live in `src/domain` and depend on
nothing else; everything outside comes in through the ports the domain
defines, so the payment provider or the database can be swapped without
touching the rules.
