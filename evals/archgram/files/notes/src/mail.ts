import { Resend } from "resend";

const resend = new Resend(process.env.RESEND_API_KEY);
export const sendDigest = () =>
  resend.emails.send({ from: "notes@example.com", to: "team@example.com", subject: "Digest", html: "" });
