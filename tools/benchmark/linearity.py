"""Fixed diagnostic schedule shared by measurement and analysis."""

# Four paired observations let analysis use a median without conditionally
# retrying a failed probe. The ABBAABBA scale order balances forward and reverse
# pairs to cancel simple monotonic host-frequency drift while remaining
# deterministic and fail closed.
LINEARITY_SEQUENCE = (
    ("n", 0),
    ("2n", 0),
    ("2n", 1),
    ("n", 1),
    ("n", 2),
    ("2n", 2),
    ("2n", 3),
    ("n", 3),
)
