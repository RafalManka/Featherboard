use crate::models::SubscriptionStatus;
use crate::plan::Plan;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BillingStatus {
    Free,
    Incomplete,
    Active,
    Trialing,
    PastDue,
    Unpaid,
    Canceled,
    Paused,
    NotConfigured,
}

impl BillingStatus {
    pub fn message(self) -> &'static str {
        match self {
            Self::Active => "Your subscription is active.",
            Self::Trialing => "Your trial is active.",
            Self::PastDue => {
                "There is a problem with your payment. Please update your billing details."
            }
            _ => "Your workspace is on the Free plan.",
        }
    }

    pub fn from_subscription_status(subscription_status: &SubscriptionStatus) -> Self {
        match subscription_status {
            SubscriptionStatus::Incomplete => Self::Incomplete,
            SubscriptionStatus::IncompleteExpired => Self::Free,
            SubscriptionStatus::Active => BillingStatus::Active,
            SubscriptionStatus::Trialing => BillingStatus::Trialing,
            SubscriptionStatus::PastDue => BillingStatus::PastDue,
            SubscriptionStatus::Unpaid => BillingStatus::Unpaid,
            SubscriptionStatus::Canceled => BillingStatus::Canceled,
            SubscriptionStatus::Paused => BillingStatus::Paused,
        }
    }

    pub fn plan(self) -> Plan {
        match self {
            Self::Active | Self::Trialing | Self::PastDue => Plan::Pro,
            Self::Free
            | Self::Incomplete
            | Self::Unpaid
            | Self::Canceled
            | Self::Paused
            | Self::NotConfigured => Plan::Free,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BillingStatus;
    use crate::models::SubscriptionStatus;
    use crate::plan::Plan;

    #[test]
    fn maps_subscription_statuses_to_billing_states_and_plans() {
        let cases = [
            (
                SubscriptionStatus::Incomplete,
                BillingStatus::Incomplete,
                Plan::Free,
            ),
            (
                SubscriptionStatus::IncompleteExpired,
                BillingStatus::Free,
                Plan::Free,
            ),
            (
                SubscriptionStatus::Trialing,
                BillingStatus::Trialing,
                Plan::Pro,
            ),
            (SubscriptionStatus::Active, BillingStatus::Active, Plan::Pro),
            (
                SubscriptionStatus::PastDue,
                BillingStatus::PastDue,
                Plan::Pro,
            ),
            (
                SubscriptionStatus::Unpaid,
                BillingStatus::Unpaid,
                Plan::Free,
            ),
            (
                SubscriptionStatus::Canceled,
                BillingStatus::Canceled,
                Plan::Free,
            ),
            (
                SubscriptionStatus::Paused,
                BillingStatus::Paused,
                Plan::Free,
            ),
        ];

        for (subscription_status, expected_status, expected_plan) in cases {
            let status = BillingStatus::from_subscription_status(&subscription_status);
            assert_eq!(status, expected_status);
            assert_eq!(status.plan(), expected_plan);
        }
    }
}
