import { Pool } from "pg";
import Stripe from "stripe";
import { withinRange } from "../common/validation";

/**
 * Start a paid subscription for an existing user.
 *
 * Entry point: POST /billing/subscriptions. Charges the user via
 * Stripe and records the subscription row in Postgres.
 */

export async function subscribeUser(
  pool: Pool,
  stripe: Stripe,
  input: { userId: string; plan: "basic" | "pro"; seats: number },
): Promise<{ subscriptionId: string }> {
  if (!withinRange(input.seats, 1, 100)) {
    throw new Error("seats must be between 1 and 100");
  }

  const sub = await stripe.subscriptions.create({
    customer: input.userId,
    items: [{ price: priceFor(input.plan) }],
    quantity: input.seats,
  } as Stripe.SubscriptionCreateParams);

  await pool.query(
    "INSERT INTO subscriptions (user_id, stripe_id, plan, seats) VALUES ($1, $2, $3, $4)",
    [input.userId, sub.id, input.plan, input.seats],
  );
  return { subscriptionId: sub.id };
}

function priceFor(plan: "basic" | "pro"): string {
  return plan === "pro" ? "price_pro" : "price_basic";
}
