#pragma once

// Logging only: keep diagnostics off the JSON stdout channel.
namespace everest_driver_shim {
struct NullStream {
    template <typename T> const NullStream& operator<<(const T&) const { return *this; }
};
inline constexpr NullStream null_stream{};
}
#define EVLOG_debug ::everest_driver_shim::null_stream
#define EVLOG_info ::everest_driver_shim::null_stream
#define EVLOG_warning ::everest_driver_shim::null_stream
#define EVLOG_error ::everest_driver_shim::null_stream
#define EVLOG_critical ::everest_driver_shim::null_stream
#define EVLOG_verbose ::everest_driver_shim::null_stream
