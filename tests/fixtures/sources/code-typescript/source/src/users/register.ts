import type { Request, Response } from "express";
import { insertUser } from "./repository";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export async function registerUser(req: Request, res: Response) {
  const { email, password } = req.body ?? {};
  if (typeof email !== "string" || !EMAIL_RE.test(email)) {
    res.status(400).json({ error: "invalid-email" });
    return;
  }
  if (typeof password !== "string" || password.length < 8) {
    res.status(400).json({ error: "weak-password" });
    return;
  }
  const user = await insertUser({ email, password });
  res.status(201).json(user);
}
