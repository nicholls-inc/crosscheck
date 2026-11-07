#pragma once

#include <ocpp/v16/ocpp_types.hpp>
#include <ocpp/v16/types.hpp>

// Constants only, copied from pinned include/ocpp/v16/smart_charging.hpp:19-23.
// profile.cpp needs no SmartChargingHandler, database, connector or transaction.
namespace ocpp::v16 {
const int DEFAULT_AND_MAX_NUMBER_PHASES = 3;
const int HOURS_PER_DAY = 24;
const int SECONDS_PER_HOUR = 3600;
const int SECONDS_PER_DAY = 86400;
const int DAYS_PER_WEEK = 7;
}
