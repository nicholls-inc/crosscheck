// SPDX-License-Identifier: Apache-2.0
// SmartChargingHandler glue adapted from EVerest (Pionix GmbH and Contributors).
#include <cmath>
#include <cstdint>
#include <iostream>
#include <limits>
#include <map>
#include <optional>
#include <regex>
#include <stdexcept>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>
#include <ocpp/common/constants.hpp>
#include <ocpp/v16/ocpp_types.hpp>
#include <ocpp/v16/profile.hpp>
#include <ocpp/v16/smart_charging.hpp>

using nlohmann::json;
using ocpp::DateTime;
using namespace ocpp::v16;

namespace {
void require(bool condition, const std::string& message) {
    if (!condition) throw std::invalid_argument(message);
}

int integer(const json& value, const std::string& name, int minimum = 0) {
    require(value.is_number_integer(), name + " must be an integer");
    require(value >= minimum && value <= std::numeric_limits<int>::max(), name + " out of range");
    return value.get<int>();
}

DateTime timestamp(const json& value) {
    const auto text = value.get<std::string>();
    static const std::regex utc(
        R"(^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|\+00:00)$)");
    require(std::regex_match(text, utc), "timestamp must be RFC3339 UTC");
    return DateTime(text);
}

ChargingProfile parse_profile(const json& value) {
    integer(value.at("chargingProfileId"), "chargingProfileId");
    integer(value.at("stackLevel"), "stackLevel");
    if (value.contains("transactionId")) integer(value.at("transactionId"), "transactionId");
    for (const auto* field : {"validFrom", "validTo"}) {
        if (value.contains(field)) timestamp(value.at(field));
    }
    const auto& schedule = value.at("chargingSchedule");
    if (schedule.contains("startSchedule")) timestamp(schedule.at("startSchedule"));
    if (schedule.contains("duration")) integer(schedule.at("duration"), "duration", 1);
    const auto& periods = schedule.at("chargingSchedulePeriod");
    require(periods.is_array() && !periods.empty(), "chargingSchedulePeriod must be a nonempty array");
    int previous = -1;
    for (const auto& period : periods) {
        const int start = integer(period.at("startPeriod"), "startPeriod");
        require(start > previous && (previous != -1 || start == 0),
                "periods must start at 0 and strictly increase");
        previous = start;
        const auto& limit = period.at("limit");
        require(limit.is_number() && std::isfinite(limit.get<float>()) && limit.get<double>() >= 0,
                "limit must be finite and nonnegative");
        if (period.contains("numberPhases")) {
            require(integer(period.at("numberPhases"), "numberPhases", 1) <= DEFAULT_AND_MAX_NUMBER_PHASES,
                    "numberPhases must be 1..3");
        }
    }
    auto profile = value.get<ChargingProfile>(); // Real upstream OCPP JSON deserializer.
    if (profile.chargingProfileKind == ChargingProfileKindType::Recurring) {
        require(profile.recurrencyKind.has_value() && profile.chargingSchedule.startSchedule.has_value(),
                "Recurring profile requires recurrencyKind and startSchedule");
    }
    if (profile.chargingProfileKind == ChargingProfileKindType::Absolute) {
        require(profile.chargingSchedule.startSchedule.has_value(), "Absolute profile requires startSchedule");
    }
    require(!profile.validFrom || !profile.validTo || *profile.validFrom <= *profile.validTo,
            "validFrom must not exceed validTo");
    return profile;
}

struct InstalledProfiles {
    std::map<int, ChargingProfile> cpmax, tx_default, tx;

    // smart_charging.cpp:417-452: add_*_profile map assignment, without DB writes/locks.
    // Profiles arrive in installation order; station defaults are copied to the connector.
    void install(int id, int connector, const ChargingProfile& profile) {
        if (id != 0 && id != connector) return;
        switch (profile.chargingProfilePurpose) {
        case ChargingProfilePurposeType::ChargePointMaxProfile:
            require(id == 0, "ChargePointMaxProfile requires connectorId 0");
            cpmax[profile.stackLevel] = profile;
            break;
        case ChargingProfilePurposeType::TxDefaultProfile:
            tx_default[profile.stackLevel] = profile;
            break;
        case ChargingProfilePurposeType::TxProfile:
            require(id > 0, "TxProfile requires a positive connectorId");
            tx[profile.stackLevel] = profile;
            break;
        }
    }

    // smart_charging.cpp:598-661: get_valid_profiles. Online: no ignored purposes.
    // Time arguments are unused upstream. No transaction ID exists in this contract,
    // so an explicitly transaction-ID-bound TxProfile cannot match.
    std::vector<ChargingProfile> valid(bool station) const {
        std::vector<ChargingProfile> output;
        for (const auto& [level, profile] : cpmax) output.push_back(profile);
        if (!station) {
            for (const auto& [level, profile] : tx) {
                if (!profile.transactionId) output.push_back(profile);
            }
            for (const auto& [level, profile] : tx_default) output.push_back(profile);
        }
        return output;
    }
};

// smart_charging.cpp:184-215: generate_evse_intermediates, faithfully retaining
// the appended station profiles (including duplicated CPMax entries).
std::vector<IntermediateProfile> generate_evse_intermediates(
    std::vector<ChargingProfile> evse_profiles, const std::vector<ChargingProfile>& station_profiles,
    const DateTime& start, const DateTime& end, std::optional<DateTime> session_start,
    bool simulate_transaction_active) {
    evse_profiles.insert(evse_profiles.end(), station_profiles.begin(), station_profiles.end());
    std::vector<IntermediateProfile> output;
    if (session_start.has_value() || simulate_transaction_active) {
        auto default_periods = calculate_all_profiles(start, end, session_start, evse_profiles,
                                                     ChargingProfilePurposeType::TxDefaultProfile);
        auto tx_periods = calculate_all_profiles(start, end, session_start, evse_profiles,
                                                ChargingProfilePurposeType::TxProfile);
        auto defaults = generate_profile_from_periods(default_periods, start, end);
        auto tx = generate_profile_from_periods(tx_periods, start, end);
        output.push_back(merge_tx_profile_with_tx_default_profile(tx, defaults));
    }
    return output;
}

json run(const json& input) {
    const auto start = timestamp(input.at("start"));
    const auto end = timestamp(input.at("end"));
    require(start <= end, "start must not exceed end");
    const auto duration = std::chrono::duration_cast<std::chrono::seconds>(
        floor_seconds(end).to_time_point() - floor_seconds(start).to_time_point()).count();
    require(duration <= std::numeric_limits<int>::max(), "window exceeds int32 seconds");
    const int connector = integer(input.at("connector"), "connector", 1);
    const auto unit_text = input.at("unit").get<std::string>();
    require(unit_text == "A" || unit_text == "W", "unit must be A or W");
    const auto unit = unit_text == "A" ? ChargingRateUnit::A : ChargingRateUnit::W;
    std::optional<DateTime> session_start;
    if (!input.at("session_start").is_null()) {
        session_start = floor_seconds(timestamp(input.at("session_start")));
    }
    InstalledProfiles installed;
    require(input.at("profiles").is_array(), "profiles must be an array");
    for (const auto& entry : input.at("profiles")) {
        installed.install(integer(entry.at("connectorId"), "connectorId"), connector,
                          parse_profile(entry.at("profile")));
    }

    // smart_charging.cpp:156-181, 245-318: CompositeScheduleConfig defaults and
    // calculate_enhanced_composite_schedule's evse_id > 0 path; is_offline=false.
    // The supplied session start stands in for the transaction's energy timestamp.
    const auto station = installed.valid(true);
    auto combined = generate_evse_intermediates(installed.valid(false), station, start, end,
                                               session_start, session_start.has_value());
    auto max_periods = calculate_all_profiles(start, end, session_start, station,
                                             ChargingProfilePurposeType::ChargePointMaxProfile);
    combined.push_back(generate_profile_from_periods(max_periods, start, end));
    const auto intermediate = merge_profiles_by_lowest_limit(combined);
    const auto default_limit = unit == ChargingRateUnit::A ? ocpp::DEFAULT_LIMIT_AMPS
                                                               : ocpp::DEFAULT_LIMIT_WATTS;
    const auto schedule = convert_intermediate_into_schedule(intermediate, unit, default_limit,
                                                             DEFAULT_AND_MAX_NUMBER_PHASES, ocpp::LOW_VOLTAGE);
    json raw = json::array(), periods = json::array();
    for (const auto& p : intermediate) {
        raw.push_back({{"startPeriod", p.startPeriod}, {"current_limit", p.current_limit},
                       {"power_limit", p.power_limit}});
    }
    for (const auto& p : schedule) {
        periods.push_back({{"startPeriod", p.startPeriod}, {"limit", p.limit},
                           {"numberPhases", p.numberPhases ? json(*p.numberPhases) : json(nullptr)}});
    }
    return {{"intermediate", raw}, {"schedule", {{"startSchedule", floor_seconds(start).to_rfc3339()},
            {"duration", elapsed_seconds(floor_seconds(end), floor_seconds(start))},
            {"chargingRateUnit", unit_text}, {"chargingSchedulePeriod", periods}}}};
}
} // namespace

int main() {
    try {
        const auto input = json::parse(std::cin); // Reject trailing JSON or non-whitespace.
        std::cout << run(input).dump() << '\n';
        return 0;
    } catch (const std::exception& error) {
        std::cout << json{{"error", error.what()}}.dump() << '\n';
        return 2;
    }
}
