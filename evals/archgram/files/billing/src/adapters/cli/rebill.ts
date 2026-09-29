import { billInvoice } from "../../app/billInvoice.js";
import { ports } from "../../wiring.js";

const [customerId, units] = process.argv.slice(2);
console.log(await billInvoice({ customerId, units: Number(units) }, ports));
