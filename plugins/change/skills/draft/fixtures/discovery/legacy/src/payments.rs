// src/payments.rs — payment flow for the legacy monolith.
//
// Surfaces two capabilities:
//   - checkout        (completes payment for a cart; depends on
//                      cart_management, emits a payment_intent)
//   - payment_intent  (wraps the payment-gateway authorisation call;
//                      depends on checkout kicking it off)

use crate::orders::Order;

pub struct PaymentIntent {
    pub id: String,
    pub order_id: u64,
    pub amount_cents: u64,
    pub status: IntentStatus,
}

pub enum IntentStatus { Requires, Authorised, Failed }

/// Checkout.
///
/// Capability: `checkout` (depends on `cart_management`).
/// Completes payment for a user's pending order: computes the
/// amount owed, asks the gateway for a payment intent, and
/// records the result on the order.
pub fn checkout(order: &Order) -> Result<PaymentIntent, &'static str> {
    let amount = compute_total(order);
    let intent = create_payment_intent(order.id, amount)?;
    record_intent(order.id, &intent);
    Ok(intent)
}

/// Payment intent.
///
/// Capability: `payment_intent` (depends on `checkout`).
/// Thin wrapper over the payment-gateway authorisation call.
/// Returns a `PaymentIntent` in the `Requires` or `Authorised`
/// state depending on the gateway response.
pub fn create_payment_intent(order_id: u64, amount_cents: u64)
    -> Result<PaymentIntent, &'static str>
{
    let gateway_resp = call_gateway(order_id, amount_cents)?;
    Ok(PaymentIntent {
        id: gateway_resp.id,
        order_id,
        amount_cents,
        status: if gateway_resp.authorised {
            IntentStatus::Authorised
        } else {
            IntentStatus::Requires
        },
    })
}

struct GatewayResp {
    id: String,
    authorised: bool,
}

fn compute_total(_o: &Order) -> u64 { unimplemented!() }
fn call_gateway(_o: u64, _a: u64) -> Result<GatewayResp, &'static str> {
    unimplemented!()
}
fn record_intent(_order_id: u64, _i: &PaymentIntent) {}
