import { Pool } from "pg";
import sgMail from "@sendgrid/mail";

/**
 * Verify a newly registered account via a one-time email token.
 *
 * Entry points:
 *   - POST /auth/verify-email — dispatch the verification email
 *     (called from the user-registration flow).
 *   - GET  /auth/verify       — consume the token and mark the
 *     user verified.
 */

const TOKEN_TTL_SECONDS = 60 * 60 * 24;

export async function sendVerificationEmail(
  pool: Pool,
  userId: string,
  email: string,
): Promise<void> {
  const token = randomToken();
  await pool.query(
    "INSERT INTO verification_tokens (user_id, token, expires_at) VALUES ($1, $2, now() + $3 * interval '1 second')",
    [userId, token, TOKEN_TTL_SECONDS],
  );
  await sgMail.send({
    to: email,
    from: "noreply@example.com",
    subject: "Verify your account",
    text: `Click to verify: https://app.example.com/auth/verify?token=${token}`,
  });
}

export async function consumeVerificationToken(
  pool: Pool,
  token: string,
): Promise<{ userId: string }> {
  const { rows } = await pool.query(
    "DELETE FROM verification_tokens WHERE token = $1 AND expires_at > now() RETURNING user_id",
    [token],
  );
  if (rows.length === 0) throw new Error("token invalid or expired");
  const userId = rows[0].user_id as string;
  await pool.query("UPDATE users SET verified = true WHERE id = $1", [userId]);
  return { userId };
}

function randomToken(): string {
  return Math.random().toString(36).slice(2);
}
