import { nonEmpty } from "../common/util";

export type OrderInput = {
  orderId: string;
  amount: number;
  currency: string;
};

export type OrderClass = "small" | "medium" | "large";

export type OrderClassification = {
  orderId: string;
  class: OrderClass;
};

const SMALL_MAX = 50;
const MEDIUM_MAX = 500;

export function classifyOrder(input: OrderInput): OrderClassification {
  if (!nonEmpty(input.orderId)) {
    throw new Error("orderId is required");
  }
  if (input.amount < 0) {
    throw new Error("amount must be non-negative");
  }

  let cls: OrderClass;
  if (input.amount <= SMALL_MAX) {
    cls = "small";
  } else if (input.amount <= MEDIUM_MAX) {
    cls = "medium";
  } else {
    cls = "large";
  }

  return { orderId: input.orderId, class: cls };
}
