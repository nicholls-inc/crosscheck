from decimal import Decimal

from m import Plan

# SAFE: module-level code with a 2dp literal
FREE = Plan(fee=Decimal("0.00"))
