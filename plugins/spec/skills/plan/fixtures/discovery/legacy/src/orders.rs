// src/orders.rs — cart + order lifecycle for the legacy monolith.
//
// Surfaces three capabilities:
//   - cart_management   (add/remove items from the active cart)
//   - order_create      (turn a cart into a pending order)
//   - order_update      (modify an existing order: quantities, address)
//
// cart_management depends on the user module (a cart is owned by a
// user row). order_create depends on cart_management (an order is
// materialised from a cart snapshot). order_update is independent
// of cart state once the order exists.

use crate::user::User;

pub struct Cart {
    pub owner: u64,
    pub items: Vec<LineItem>,
}

pub struct LineItem {
    pub sku: String,
    pub qty: u32,
}

pub struct Order {
    pub id: u64,
    pub owner: u64,
    pub items: Vec<LineItem>,
    pub status: OrderStatus,
}

pub enum OrderStatus { Pending, Placed, Cancelled }

/// Cart management.
///
/// Capability: `cart_management` (depends on `registration`).
/// Add/remove items on a user's active cart.
pub fn add_to_cart(user: &User, sku: &str, qty: u32) -> Cart {
    let mut cart = load_cart(user.id);
    cart.items.push(LineItem { sku: sku.to_string(), qty });
    save_cart(&cart);
    cart
}

pub fn remove_from_cart(user: &User, sku: &str) -> Cart {
    let mut cart = load_cart(user.id);
    cart.items.retain(|li| li.sku != sku);
    save_cart(&cart);
    cart
}

/// Order creation.
///
/// Capability: `order_create` (depends on `cart_management`).
/// Snapshot the user's active cart into a new `Order` with
/// `status = Pending`. Clears the cart.
pub fn create_order(user: &User) -> Order {
    let cart = load_cart(user.id);
    let order = Order {
        id: next_order_id(),
        owner: user.id,
        items: cart.items.clone(),
        status: OrderStatus::Pending,
    };
    save_order(&order);
    clear_cart(user.id);
    order
}

/// Order update.
///
/// Capability: `order_update`.
/// Modify quantities or shipping details on an existing order.
pub fn update_order(order_id: u64, patch: OrderPatch) -> Order {
    let mut order = load_order(order_id);
    if let Some(items) = patch.items {
        order.items = items;
    }
    save_order(&order);
    order
}

pub struct OrderPatch { pub items: Option<Vec<LineItem>> }

fn load_cart(_owner: u64) -> Cart { unimplemented!() }
fn save_cart(_c: &Cart) {}
fn clear_cart(_owner: u64) {}
fn next_order_id() -> u64 { unimplemented!() }
fn load_order(_id: u64) -> Order { unimplemented!() }
fn save_order(_o: &Order) {}
