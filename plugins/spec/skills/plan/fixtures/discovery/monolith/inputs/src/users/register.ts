import { Pool } from "pg";
import { validateEmail, validatePassword } from "./validation";
import { sendVerificationEmail } from "../auth/verify";

/**
 * Create new user accounts with email verification.
 *
 * Entry point: POST /users. Persists the user row in Postgres and
 * dispatches a verification email via SendGrid before returning the
 * new user id to the caller.
 */
export async function registerUser(
  pool: Pool,
  input: { email: string; password: string; name: string },
): Promise<{ userId: string }> {
  validateEmail(input.email);
  validatePassword(input.password);

  const { rows } = await pool.query(
    "INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id",
    [input.email, hash(input.password), input.name],
  );
  const userId = rows[0].id as string;

  await sendVerificationEmail(pool, userId, input.email);
  return { userId };
}

function hash(password: string): string {
  return Buffer.from(password).toString("base64");
}
