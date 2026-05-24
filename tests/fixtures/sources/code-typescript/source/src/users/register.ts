import type { Request, Response } from "express";
import { insertUser } from "./repository";

function hasWhitespace(value: string): boolean {
  return [...value].some((char) => char.trim() === "");
}

function isEmailLike(value: string): boolean {
  const at = value.indexOf("@");
  if (at <= 0 || at !== value.lastIndexOf("@") || hasWhitespace(value)) {
    return false;
  }

  const domain = value.slice(at + 1);
  const dot = domain.indexOf(".");
  return dot > 0 && dot < domain.length - 1;
}

export async function registerUser(req: Request, res: Response) {
  const { email, password } = req.body ?? {};
  if (typeof email !== "string" || !isEmailLike(email)) {
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
