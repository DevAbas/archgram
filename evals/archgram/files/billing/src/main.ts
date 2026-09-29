import express from "express";
import { routes } from "./adapters/http/routes.js";

express().use(routes).listen(3000);
