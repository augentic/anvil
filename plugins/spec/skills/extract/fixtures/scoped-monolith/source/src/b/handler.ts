import { nonEmpty } from "../common/util";

export type CustomerInput = {
  customerId: string;
  yearsActive: number;
};

export type LoyaltyTier = "bronze" | "silver" | "gold";

export type LoyaltyResult = {
  customerId: string;
  tier: LoyaltyTier;
};

export function computeLoyaltyTier(input: CustomerInput): LoyaltyResult {
  if (!nonEmpty(input.customerId)) {
    throw new Error("customerId is required");
  }
  if (input.yearsActive < 0) {
    throw new Error("yearsActive must be non-negative");
  }

  let tier: LoyaltyTier;
  if (input.yearsActive < 1) {
    tier = "bronze";
  } else if (input.yearsActive < 5) {
    tier = "silver";
  } else {
    tier = "gold";
  }

  return { customerId: input.customerId, tier };
}
