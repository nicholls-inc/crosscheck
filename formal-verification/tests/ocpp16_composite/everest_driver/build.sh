#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
driver_dir="$PWD"
mkdir -p .deps build
# Keep compiler scratch files inside the repository as well.
export TMPDIR="$driver_dir/build"
revision=dd696165d407d461a724232a4d77fe476a964ef1

download() {
    local url="$1" dest="$2"
    if [[ ! -s "$dest" ]]; then
        curl --fail --location --retry 3 "$url" -o "$dest.part"
        mv "$dest.part" "$dest"
    fi
}

if [[ ! -d ".deps/EVerest-$revision" ]]; then
    download "https://codeload.github.com/EVerest/EVerest/tar.gz/$revision" .deps/everest.tar.gz
    tar -xzf .deps/everest.tar.gz -C .deps
fi
if [[ ! -d .deps/date-3.0.1 ]]; then
    download https://codeload.github.com/HowardHinnant/date/tar.gz/v3.0.1 .deps/date.tar.gz
    tar -xzf .deps/date.tar.gz -C .deps
fi
mkdir -p .deps/include/nlohmann
download https://raw.githubusercontent.com/nlohmann/json/v3.11.3/single_include/nlohmann/json.hpp .deps/include/nlohmann/json.hpp
download https://raw.githubusercontent.com/nlohmann/json/v3.11.3/single_include/nlohmann/json_fwd.hpp .deps/include/nlohmann/json_fwd.hpp

ocpp="$driver_dir/.deps/EVerest-$revision/lib/everest/ocpp"
date_dir="$driver_dir/.deps/date-3.0.1"
compiler="${CXX:-c++}"
includes=(-I"$driver_dir/shim" -I"$ocpp/include" -I"$driver_dir/.deps/include"
          -I"$date_dir/include" -I/opt/homebrew/include)
flags=(-std=c++17 -O2 -DUSE_OS_TZDB=1 -DHAS_REMOTE_API=0 -DFMT_HEADER_ONLY
       -ffunction-sections -fdata-sections -pthread)
if [[ "$(uname -s)" == Darwin ]]; then
    link_flags=(-Wl,-dead_strip)
else
    link_flags=(-Wl,--gc-sections)
fi
# Compile complete upstream translation units; discard unused code at link time.
sources=("$driver_dir/main.cpp" "$ocpp/lib/ocpp/v16/profile.cpp"
         "$ocpp/lib/ocpp/v16/ocpp_types.cpp" "$ocpp/lib/ocpp/v16/ocpp_enums.cpp"
         "$ocpp/lib/ocpp/common/types.cpp" "$date_dir/src/tz.cpp")
objects=()
for index in "${!sources[@]}"; do
    object="$driver_dir/build/$index.o"
    "$compiler" "${flags[@]}" "${includes[@]}" -c "${sources[$index]}" -o "$object"
    objects+=("$object")
done
"$compiler" -pthread "${link_flags[@]}" "${objects[@]}" -o build/everest_composite
echo "Built $driver_dir/build/everest_composite"
